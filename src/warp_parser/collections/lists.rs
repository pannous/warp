//! Lists: bracketed groups, separators and grouping by them, signed numbers starting a list, arrow bodies in lists
use super::*;
use crate::lowering::nodes::is_block;

/// `(x => print x)`: `=>` binds looser than the space, so the words after a lambda's arrow are its body (`print x`), not
/// further items; a run of entries `1 => "a" 2 => "b"` stays a list of pairs
pub(crate) fn with_arrow_bodies(mut items: Vec<Node>) -> Vec<Node> {
	let is_arrow = |node: &Node| matches!(node.drop_meta(), Node::Key(_, Op::FatArrow, _));
	let Some(arrow) = items.iter().position(is_arrow) else {
		return items;
	};
	if arrow + 1 == items.len() || items[arrow + 1..].iter().any(is_arrow) {
		return items;
	}
	let rest = items.split_off(arrow + 1);
	let Node::Key(parameters, op, body) = items.pop().expect("the arrow").drop_meta().clone() else {
		unreachable!("an arrow")
	};
	let body = one_expression(&[vec![*body], rest].concat());
	items.push(Node::Key(parameters, op, Box::new(body)));
	items
}

impl WarpParser {
	/// `1 -1`, `[1 +1]`: a number, a space, then a sign glued to a number (D13). Asked: the list `[1 -1]` (the default,
	/// a warning when unanswered) or the arithmetic `1 - 1`. Variables, calls and words keep subtracting: `x -1`.
	/// Inside a list literal a space and a glued sign start the next item after anything (decision list-sign):
	/// `a=5; [a -1]` is [5 -1], while `[a - 1]` and `[a-1]` subtract
	pub(crate) fn signed_number_starts_a_list(&mut self, lhs: &Node) -> Result<bool, Node> {
		if self.signed_list_element == Some(self.pos) {
			return Ok(true); // answered for the operand inside
		}
		if self.in_spaced_list && self.prev_char().is_whitespace() && self.sign_glued_to_operand() {
			self.signed_list_element = Some(self.pos);
			return Ok(true);
		}
		if !matches!(lhs.drop_meta(), Node::Number(_)) || !self.prev_char().is_whitespace() || !self.number_starts_at(1)
		{
			return Ok(false);
		}
		let signed: String = self.chars[self.pos..]
			.iter()
			.take_while(|ch| !ch.is_whitespace() && !matches!(ch, ']' | ')' | '}' | ',' | ';'))
			.collect();
		let (first, sign, number) = (lhs.serialize(), self.current_char(), &signed[1..]);
		let written = format!("{first} {signed}");
		let readings = vec![
			crate::errors::diagnostic::reading("the list", &format!("{first} ({signed})")), // also inside brackets: `[1 (-1) 2]`
			crate::errors::diagnostic::reading("arithmetic", &format!("{first} {sign} {number}")),
		];
		let question = crate::errors::diagnostic::Ask::new(
			SIGNED_OPERAND_TOPIC,
			format!("is `{written}` a list or arithmetic?"),
			readings,
			crate::errors::diagnostic::Fallback::Warning,
		)
		.written(&written)
		.at(self.line_nr, self.column);
		let is_list = crate::errors::diagnostic::ask(&question)? == 0;
		if is_list {
			self.signed_list_element = Some(self.pos);
		}
		Ok(is_list)
	}

	/// `-1`, `-x`, `+(…)`: the sign at the cursor is glued to the operand after it, no operator like `->` or `-=`
	fn sign_glued_to_operand(&self) -> bool {
		let next = self.peek_char(1);
		next.is_alphanumeric() || matches!(next, '(' | '[' | '"' | '\'' | '.' | '_' | '√')
	}

	/// At the closing bracket `close` (or the end of the input), or at the end of a bracketless group
	fn at_group_end(&self, close: Option<char>) -> bool {
		match close {
			Some(closer) => matches!(self.current_char(), '\0') || self.current_char() == closer,
			None => self.end_of_input() || self.at_block_close(),
		}
	}

	pub(crate) fn parse_bracketed(&mut self, open: char) -> Node {
		let (close, bracket_type) = match open {
			'(' => (')', Bracket::Round),
			'[' => (']', Bracket::Square),
			'{' => ('}', Bracket::Curly),
			'<' => ('>', Bracket::Round),
			_ => panic!("Invalid bracket: {}", open),
		};
		let start = self.get_position();
		let outer_start = std::mem::replace(&mut self.group_start, start);
		// `def area(w, h) { w /h }`: the head's parameters are in scope inside its block (card path-parameter-scope)
		let outer_scope = self.parameters_in_scope.len();
		if bracket_type == Bracket::Curly {
			let parameters = self.parameters_before_block(self.pos);
			self.parameters_in_scope.extend(parameters);
		}
		self.advance(); // skip opening bracket
		let compares = self.equals_compares && bracket_type != Bracket::Curly; // a block is not the condition
																		 // `for i in (0 until n)` and `(0..n-1)` are still the header; a block or a list inside it is not
		let inner_header = self.in_for_header && bracket_type == Bracket::Round;
		let outer_header = std::mem::replace(&mut self.in_for_header, inner_header);
		let spaced_list = bracket_type == Bracket::Square && !self.group_holds_comma();
		let outer_spaced_list = std::mem::replace(&mut self.in_spaced_list, spaced_list);
		let list =
			self.with_equals_comparing(compares, |parser| parser.parse_list_with_separators(Some(close), bracket_type));
		self.parameters_in_scope.truncate(outer_scope);
		self.in_spaced_list = outer_spaced_list;
		self.in_for_header = outer_header;
		self.group_start = outer_start;
		list
	}

	pub(crate) fn parse_list_with_separators(&mut self, close: Option<char>, bracket: Bracket) -> Node {
		let tag_block = std::mem::take(&mut self.tag_block);
		// Collect all items with their following separators
		let mut items_with_seps: Vec<(Node, Separator)> = Vec::new();
		let scope_at_entry = self.parameters_in_scope.len();

		loop {
			// a definition's parameters are in scope until its statement ends (card path-parameter-scope)
			if items_with_seps.last().is_none_or(|(_, separator)| *separator != Separator::Space) {
				self.parameters_in_scope.truncate(scope_at_entry);
			}
			// Skip whitespace but track indent for dedent detection
			let (had_newline, line_indent) = self.skip_whitespace();

			// Check for dedent - exit this block if we're back to lower indent
			if had_newline && line_indent < self.base_indent && bracket == Bracket::None {
				break;
			}

			// Check for end condition (also check for end-of-input to avoid infinite loop)
			let ch = self.current_char();
			let at_end = self.at_group_end(close);
			if at_end {
				if let Some(closer) = close.filter(|_| ch == '\0') {
					let (line, column) = self.group_start;
					let message = format!(
						"`{closer}` is missing: the group opened at {line}:{column} runs to the end of the input"
					);
					return missing_closer(message, closer, crate::errors::fixits::end_of_source(&self.input));
				}
				if close.is_some() {
					self.advance(); // consume closing bracket
				}
				break;
			}

			let pos_before = self.pos;
			// an argument of a braceless call at statement level (`sleep 1s`): `and print "x"` after it starts the next statement
			let statement_start = items_with_seps
				.iter()
				.rposition(|(_, separator)| *separator != Separator::Space)
				.map_or(0, |last| last + 1);
			let in_command =
				items_with_seps.get(statement_start).is_some_and(|(first, _)| matches!(first.drop_meta(), Symbol(_)));
			let outer_command = std::mem::replace(&mut self.in_command, in_command);
			// `square xs |> sum` is `sum(square xs)`: the call of a function on its one argument is the piped value
			let calls_function = in_command
				&& items_with_seps.len() == statement_start + 1
				&& matches!(items_with_seps[statement_start].0.drop_meta(), Symbol(name) if crate::type_name_matching::names_a_function(name));
			let outer_pipe = std::mem::replace(&mut self.pipe_takes_call, calls_function);
			// `ul { li: "a" li: "b" }`: a block after a name and a blank is that tag's block
			self.tag_block = ch == '{'
				&& matches!(items_with_seps.last(), Some((previous, Separator::Space)) if matches!(previous.drop_meta(), Symbol(_)));
			let item_position = self.get_position();
			let item = self.parse_value();
			self.tag_block = false;
			self.pipe_takes_call = outer_pipe;
			self.in_command = outer_command;
			let item = if calls_function && {
				self.skip_spaces_and_inline_comments();
				self.at_pipeline()
			} {
				let (function, _) = items_with_seps.pop().expect("the function before its argument");
				self.continue_expr(Node::List(vec![function, item], Bracket::None, Separator::Space), 0)
			} else {
				item
			};

			let consumed_input = self.pos != pos_before;
			// `==` is loose (false equals ø): only a real ø is skipped; `sum []` keeps its empty argument
			let is_argument = in_command && items_with_seps.len() > statement_start;
			if matches!(item, Empty) && !(consumed_input && (close.is_some() || is_argument)) {
				if !consumed_input {
					self.advance();
				}
				continue;
			}

			// `def f(n)` and the lines indented below it by spaces, as `def f(n):` (by tabs the rule below takes them); Ruby's
			// `def f(n) … end` is read by its end
			let defines = items_with_seps
				.get(statement_start)
				.is_some_and(|(first, _)| matches!(first.drop_meta(), Symbol(word) if is_function_keyword(word)));
			if defines {
				self.bind_parameters(&item, false);
			}
			let item = if defines { self.with_definition_block(item) } else { item };
			let (line, column) = item_position;
			let item = match is_argument
				.then(|| self.argument_cast_reading(&items_with_seps[statement_start].0, &item, line, column))
			{
				Some(Err(unanswered)) => unanswered,
				_ => item,
			};
			let item = match self.take_trailing_comment() {
				Some(comment) => item.with_comment(comment),
				None => item,
			};
			let (had_newline, line_indent, comment) = self.skip_whitespace_and_comments();
			self.pending_comment = comment;

			// Handle indentation-based blocks
			let item = if had_newline && line_indent > self.base_indent && bracket == Bracket::None {
				// Indented block follows - parse it as body of current item
				let old_indent = self.base_indent;
				self.base_indent = line_indent;
				let body = self.parse_list_with_separators(None, Bracket::None);
				self.base_indent = old_indent;
				// Combine item with indented body as Key; a call keeps its parameters (Ruby's `def f(x)` and its lines)
				let head = match item.drop_meta() {
					Node::List(call, Bracket::Round, _)
						if matches!(call.first().map(Node::drop_meta), Some(Symbol(_))) =>
					{
						item
					}
					_ => Symbol(item.name()),
				};
				Node::Key(Box::new(head), Op::Colon, Box::new(body))
			} else if had_newline && line_indent < self.base_indent && bracket == Bracket::None {
				// Dedent - push item and exit this level
				items_with_seps.push((item, Separator::None));
				break;
			} else if bracket == Bracket::Curly {
				field_holding_lambda(item)
			} else {
				item
			};

			// Determine separator after this item
			let sep = self.parse_separator(close, had_newline, in_command || is_command(&item));

			let item = match items_with_seps.last() {
				Some((previous, Separator::Space)) if assigned_method_call(previous) && is_block(&item) => {
					let (previous, _) = items_with_seps.pop().expect("guarded");
					with_trailing_block(previous, item)
				}
				_ => item,
			};
			match declarations_of_several_globals(&item) {
				// `global a, b`: its declarations are statements of this block
				Some(declarations) => {
					items_with_seps.extend(declarations.into_iter().map(|declaration| (declaration, sep.clone())))
				}
				None => items_with_seps.push((item, sep)),
			}

			if self.pos == pos_before {
				self.advance();
			}
		}

		self.parameters_in_scope.truncate(scope_at_entry);
		self.pending_comment = None; // a comment closing a list documents nothing after it
		let list = self.group_by_separators(items_with_seps, bracket);
		match list.duplicate_key().filter(|_| !tag_block) {
			Some(key) => error(&format!("duplicate key '{}'", key)),
			None => list,
		}
	}

	/// The separator after a list item, consumed: `,` `;` a newline, a blank, or an `and` starting the next command
	fn parse_separator(&mut self, close: Option<char>, had_newline: bool, in_command: bool) -> Separator {
		match self.current_char() {
			_ if self.at_group_end(close) => Separator::None,
			',' => {
				self.advance();
				Separator::Colon
			}
			';' => {
				self.advance();
				if self.only_blanks_before_newline() {
					Separator::Newline
				} else {
					Separator::Semicolon
				}
			}
			_ if had_newline => Separator::Newline,
			_ if in_command && self.and_starts_statement() => {
				self.advance_by(AND_KEYWORD.len());
				Separator::Semicolon
			}
			_ => Separator::Space,
		}
	}

	pub(crate) fn group_by_separators(&self, items_with_seps: Vec<(Node, Separator)>, bracket: Bracket) -> Node {
		if items_with_seps.is_empty() {
			return if bracket == Bracket::Curly { Node::List(Vec::new(), bracket, Separator::None) } else { Empty };
			// an empty block is a value
		}

		// only implicit groupings unwrap a single item: explicit brackets like {x} or [x] keep the wrapper
		if items_with_seps.len() == 1 && bracket == Bracket::None {
			return only(items_with_seps).0;
		}
		// the loosest separator splits first, the groups between split by the tighter ones (None ends the list)
		let Some(loosest) =
			items_with_seps.iter().map(|(_, sep)| sep.precedence()).filter(|&precedence| precedence < 255).max()
		else {
			return grouped_list(
				items_with_seps.into_iter().map(|(node, _)| node).collect(),
				bracket,
				Separator::Space,
			);
		};
		let split_sep = items_with_seps
			.iter()
			.map(|(_, sep)| sep)
			.find(|sep| sep.precedence() == loosest)
			.cloned()
			.expect("the loosest is among them");
		let mut groups: Vec<Vec<(Node, Separator)>> = Vec::new();
		let mut current_group = Vec::new();
		for (item, sep) in items_with_seps {
			if sep.precedence() == loosest {
				current_group.push((item, Separator::None));
				groups.push(std::mem::take(&mut current_group));
			} else {
				current_group.push((item, sep));
			}
		}
		if !current_group.is_empty() {
			groups.push(current_group);
		}
		// inner groups take no brackets, which would add braces in serialization
		let grouped_nodes: Vec<Node> = groups
			.into_iter()
			.map(|group| match group.as_slice() {
				[(_, Separator::None)] => only(group).0,
				_ => self.group_by_separators(group, Bracket::None),
			})
			.collect();
		if grouped_nodes.len() == 1 && bracket == Bracket::None {
			only(grouped_nodes)
		} else {
			grouped_list(grouped_nodes, bracket, split_sep)
		}
	}
}
