//! Atoms: symbols with suffixes, definitions and their bodies, indented and end blocks, URLs, type declarations

use super::*;

const VOID_WORD: &str = "void";
/// `none` is the null ø; called, `none(xs, f)` (Kotlin's "no element matches") is an error naming it (P185)
const NONE_WORD: &str = "none";
const NONE_CALL_ERROR: &str = "none is the null ø, not a function: for \"no element matches\" write not any(xs, f)";
const STYLE_WORD: &str = "style";
/// What follows the name of a lowercase type declaration: `type point {…}`, `type size = u32`, `type pair<T>`
const TYPE_BODY_STARTS: [char; 4] = ['{', '=', ':', '<'];
/// The first argument of a block, what Elixir's `&1` and the element of Ruby's `&:to_s` are
const CAPTURED_ARGUMENT: &str = "$0";
/// The most code points an emoji in code may have (a family ZWJ sequence has 7 to 11, a subdivision flag 7)
const LONGEST_EMOJI: usize = 32;

/// `T, F: Fn(i32) -> i32` → `T F`: the names before each bound
fn type_parameter_names(written: &str) -> Vec<String> {
	let mut depth = 0;
	let mut parts = vec![String::new()];
	for ch in written.chars() {
		match ch {
			'<' | '(' => depth += 1,
			'>' | ')' => depth -= 1,
			',' if depth == 0 => parts.push(String::new()),
			_ => {}
		}
		if ch != ',' || depth != 0 {
			parts.last_mut().expect("one part").push(ch);
		}
	}
	parts.iter().filter_map(|part| part.split(':').next().map(str::trim).filter(|name| !name.is_empty()).map(str::to_string)).collect()
}

impl WarpParser {
	/// Parse an atomic expression (no infix operators)
	/// Handles: numbers, strings, brackets, symbols with named blocks
	pub(super) fn parse_atom(&mut self) -> Node {
		let (_, _, comment) = self.skip_whitespace_and_comments();
		let (line_nr, column) = self.get_position();

		if self.is_at_line_end() {
			return Empty;
		}

		let node = match self.current_char() {
			// `&name` is the function `name` itself: a name is already a function value where a function is expected
			'&' if self.starts_function_reference() => {
				self.advance();
				match self.parse_symbol() {
					Ok(name) => crate::closures::function_reference(name),
					Err(message) => error(&message),
				}
			}
			// Elixir's capture `&(&1 * 2)`: the block `{$0 * 2}`; its `&1`, `&2` are `$0`, `$1`
			'&' if self.starts_capture() => {
				self.advance();
				let outer = std::mem::replace(&mut self.in_capture, true);
				let group = self.parse_bracketed('(');
				self.in_capture = outer;
				match group {
					Node::List(items, Bracket::Round, separator) => Node::List(items, Bracket::Curly, separator),
					other => Node::List(vec![other], Bracket::Curly, Separator::None),
				}
			}
			'&' if self.peek_char(1).is_ascii_digit() && self.in_capture => {
				self.advance();
				let position: String = (0..).map(|at| self.peek_char(at)).take_while(char::is_ascii_digit).collect();
				self.advance_by(position.len());
				let position: usize = position.parse().unwrap_or(1);
				Node::Symbol(format!("${}", position.saturating_sub(1)))
			}
			// Ruby's `&:to_s`: the block `{$0.to_s}`, the method called on each element
			'&' if self.peek_char(1) == ':' && self.peek_char(2).is_alphabetic() => {
				self.advance_by(2);
				match self.parse_symbol() {
					Ok(method) => {
						let call = Node::Key(Box::new(Node::Symbol(CAPTURED_ARGUMENT.to_string())), Op::Dot, Box::new(Node::Symbol(method)));
						Node::List(vec![call], Bracket::Curly, Separator::None)
					}
					Err(message) => error(&message),
				}
			}
			'"' | '\'' | '«' | '`' => self.parse_string(),
			// `a, *rest = xs`: the starred name takes the items the other names leave (src/lowering/tuples.rs); `...rest`
			// (JS) is the starred `*rest` too: a rest parameter or a spread argument (src/lowering/variadic.rs)
			'.' if self.peek_char(1) == '.' && self.peek_char(2) == '.' && self.is_identifier_start(3) => self.parse_starred(3),
			// Python's `**kw`: the keyword arguments as one object (src/lowering/variadic.rs)
			'*' if self.peek_char(1) == '*' && self.is_identifier_start(2) => match self.parse_starred(2) {
				Node::Symbol(name) => Node::Symbol(format!("{}{name}", crate::tuples::STARRED)),
				other => other,
			},
			'*' if self.is_identifier_start(1) => self.parse_starred(1),
			// `.card { … }`: a leading-dot name, a class selector in a style block (style_rules.rs)
			'.' if self.is_identifier_start(1) => {
				self.advance();
				match self.parse_symbol() {
					Ok(name) => Node::Symbol(format!(".{name}")),
					Err(message) => error(&message),
				}
			}
			'-' if self.peek_char(1) == '>' && !self.options.data_mode => self.parse_stabby_lambda(),
			'(' | '[' | '{' => self.parse_bracketed(self.current_char()),
			'<' if self.options.xml_mode => self.parse_xml_tag(),
			'<' => self.parse_bracketed('<'),
			';' | '>' | '}' | ')' | ']' => Empty, // Closing brackets/terminators handled by caller
			'ø' => { self.advance(); return Empty }
			'∞' => { self.advance(); return Node::Number(Number::Inf) } // the float infinity (P56)
			// $n parameter reference (e.g., $0 = first param)
			'@' if self.peek_char(1).is_alphabetic() => self.parse_attribute(),
			'@' if self.peek_char(1) == '(' && !self.options.data_mode => self.parse_matlab_lambda(),
			'\\' if self.backslash_lambda_ahead() && !self.options.data_mode => self.parse_backslash_lambda(),
			// `@1`: a reference to the node whose @id is 1 (wiki/reference.md, P160)
			'@' if self.peek_char(1).is_ascii_digit() => {
				self.advance(); // skip '@'
				let mut digits = String::new();
				while self.current_char().is_ascii_digit() {
					digits.push(self.current_char());
					self.advance();
				}
				Node::Symbol(format!("{ATTRIBUTE_MARK}{digits}"))
			}
			'$' if self.peek_char(1).is_alphabetic() || self.peek_char(1) == '_' => self.parse_dollar_name(),
			'$' if self.peek_char(1).is_numeric() => {
				self.advance(); // skip '$'
				let mut num_str = String::new();
				while self.current_char().is_numeric() {
					num_str.push(self.current_char());
					self.advance();
				}
				Node::Symbol(format!("${}", num_str))
			}
			ch if ch.is_numeric() || self.number_starts_at(0) || (ch == '-' && self.number_starts_at(1)) => {
				self.parse_number()
			}
			ch if ch.is_alphabetic() || ch == '_' => {
				let atom = self.parse_elixir_function().or_else(|| self.parse_python_lambda()).or_else(|| self.parse_f_string()).unwrap_or_else(|| self.parse_symbol_with_suffix());
				// the function's own atom takes the mark, not an atom parsed inside it (its parameters)
				let head = match atom.drop_meta() {
					Node::List(items, _, _) => items.first().map(Node::name),
					other => Some(other.name()),
				};
				match self.generic_names.take() {
					Some((name, names)) if head.as_ref() == Some(&name) => Node::Meta { node: Box::new(atom), data: Box::new(Node::key(crate::welcome_forms::GENERIC_MARK, Node::Text(names.join(" ")))) },
					pending => {
						self.generic_names = pending;
						atom
					}
				}
			}
			'\\' if let Some((name, length)) = crate::uniscript_entities::entity_name_at(&self.chars, self.pos) => {
				let warned = self.warn_unknown_entity(&name, self.column);
				(0..length).for_each(|_| self.advance());
				warned.map_or_else(|strict| strict, |()| Node::Text(format!("\\:{name}")))
			}
			'\\' if let Some(name) = crate::uniscript_entities::bare_entity_name_at(&self.chars, self.pos) => {
				(0..=name.len()).for_each(|_| self.advance());
				error(&format!("a uniscript entity is written \\:{name}, not \\{name}"))
			}
			_ if let Some((operator, length)) = self.bare_operator_in_block() => {
				(0..length).for_each(|_| self.advance());
				error(&format!("{{{operator}}} is an operator without operands, no function: write them, e.g. map xs {{it + 1}}"))
			}
			ch if crate::extensions::strings::starts_an_emoji(ch) => self.parse_emoji(),
			ch => {
				warn!(
					"Unexpected character '{}' at line {}, column {}",
					ch, line_nr, column
				);
				self.advance();
				error(&format!("Unexpected character '{}'", ch))
			}
		};

		if node == Empty {
			return node;
		}

		// Attach metadata
		let node = node.with_meta_data(LineInfo {
			line_nr,
			column,
			#[cfg(debug_assertions)]
			line: self.current_line.clone(),
		});
		if let Some(c) = comment {
			node.with_comment(c)
		} else {
			node
		}
	}

	/// The word of `length` characters just parsed is the first thing of its statement: nothing, a newline, `;` or `{` before it
	pub(super) fn word_starts_statement(&self, length: usize) -> bool {
		let word_start = self.pos.saturating_sub(length);
		let before = self.chars[..word_start].iter().rev().find(|ch| !matches!(ch, ' ' | '\t' | '\r'));
		matches!(before, None | Some('\n' | ';' | '{'))
	}

	/// `to name params: body`, after the word `to`: the function `name(params) := body`.
	/// The body is the rest of the statement, a `{…}` block or an indented block. Parameters are plain names or, with a
	/// known type word, typed slots (`to square a number:`, type_name_matching::parameter_slots). Anything else is no definition.
	pub(super) fn try_parse_to_definition(&mut self) -> Option<Node> {
		let before_header = (self.pos, self.line_nr, self.column, self.current_line.clone());
		let restore = |parser: &mut Self| (parser.pos, parser.line_nr, parser.column, parser.current_line) = before_header.clone();
		self.skip_spaces();
		let Some(name) = self.at_identifier_start().then(|| self.parse_symbol().ok()).flatten() else {
			restore(self);
			return None;
		};
		let mut parameters = Vec::new();
		// `to call person{name?, phone number} do …` (wiki/argument.md): the parameter's fields, glued to it
		let mut shapes = Vec::new();
		loop {
			self.skip_spaces();
			// `to square a number: …` or `to square a number { … }`
			if (self.current_char() == ':' && self.peek_char(1) != '=') || self.current_char() == '{' {
				break;
			}
			match self.at_identifier_start().then(|| self.parse_symbol().ok()).flatten() {
				Some(word) if word == DO_WORD && !parameters.is_empty() => break,
				Some(parameter) => {
					if self.current_char() == '{' {
						shapes.push((parameter.clone(), self.parse_atom()));
					}
					parameters.push(parameter)
				}
				None => {
					restore(self);
					return None;
				}
			}
		}
		if self.current_char() == ':' {
			self.advance();
			self.skip_spaces();
		}
		let body = self.parse_definition_body();
		self.functions.insert(name.clone());
		let words: Vec<&str> = parameters.iter().map(String::as_str).collect();
		let declared_types = &self.declared_types;
		let is_known_type = |word: &str| {
			crate::analyzer::type_word_kind(word).is_some() || crate::analyzer::plural_element_type(word).is_some() || declared_types.contains(word)
		};
		let (parameters, body) = match crate::type_name_matching::parameter_slots(&words, &body, &is_known_type) {
			Ok(slots) => slots,
			Err(message) => return Some(error(&message)),
		};
		let parameters: Vec<Node> = parameters.into_iter().map(|parameter| match shapes.iter().find(|(name, _)| *name == parameter.name()) {
			Some((_, fields)) => Node::Key(Box::new(parameter), Op::Colon, Box::new(fields.clone())),
			None => parameter,
		}).collect();
		if let Some(clash) = self.phrase_redefinition(&name, &parameters) {
			return Some(clash);
		}
		let head = if parameters.is_empty() {
			Symbol(name)
		} else {
			self.functions_with_parameters.insert(name.clone());
			self.phrase_definitions.insert(name.clone(), parameters.clone());
			let head = Node::List([vec![Symbol(name)], parameters].concat(), Bracket::Round, Separator::None);
			// P52: the prepositions of the phrase, for phrase_calls to read calls like `add 1 to 2`
			let pattern = crate::phrase_calls::pattern_text(&words);
			// `To square a number: …` of one slot is called in English as `square of x` too (`square 3` stays a call)
			let pattern = if pattern == crate::phrase_calls::SLOT { format!("{OF_WORD} {pattern}") } else { pattern };
			if pattern.split(' ').any(|part| part != crate::phrase_calls::SLOT) {
				Node::Meta { node: Box::new(head), data: Box::new(Node::key(crate::phrase_calls::PHRASE_MARK, Node::Text(pattern))) }
			} else {
				head
			}
		};
		Some(Node::Key(Box::new(head), Op::Define, Box::new(body)))
	}

	/// `to kill a person: …` after `to kill a dog: …`: the same phrase with other untyped nouns can only be told apart by types
	pub(super) fn phrase_redefinition(&self, name: &str, parameters: &[Node]) -> Option<Node> {
		let earlier = self.phrase_definitions.get(name).filter(|earlier| earlier.len() == parameters.len())?;
		let untyped = |parameter: &Node| matches!(parameter.drop_meta(), Symbol(_));
		let differing: Vec<(&Node, &Node)> = earlier.iter().zip(parameters).filter(|(before, now)| before != now).collect();
		if differing.is_empty() || !differing.iter().all(|(before, now)| untyped(before) && untyped(now)) {
			return None;
		}
		let classes: Vec<String> = differing.iter().flat_map(|(before, now)| [before.name(), now.name()]).map(|noun| format!("class {noun}")).collect();
		Some(error(&format!("{name} is defined twice; declare {} to dispatch on them", classes.join(" and "))))
	}

	/// Move back to the end of the last thing before the whitespace run behind the cursor
	pub(super) fn seek_back_over_whitespace(&mut self) {
		let mut position = self.pos.min(self.chars.len());
		while position > 0 && self.chars[position - 1].is_whitespace() {
			position -= 1;
		}
		let line_start = self.chars[..position].iter().rposition(|ch| *ch == '\n').map_or(0, |newline| newline + 1);
		self.pos = position;
		self.line_nr = 1 + self.chars[..position].iter().filter(|ch| **ch == '\n').count();
		self.column = position - line_start + 1;
		self.current_line = self.chars[line_start..].iter().take_while(|ch| **ch != '\n').collect();
	}

	/// The rest of the line, or the indented lines below it, or a `{…}` block
	pub(super) fn parse_definition_body(&mut self) -> Node {
		if self.current_char() == '{' {
			return self.parse_atom();
		}
		if !self.only_blanks_before_newline() {
			return self.rest_of_statement(); // `to mail x to address: print address`
		}
		self.parse_indented_block().unwrap_or_else(|| error("a definition needs a body: `to name params: body`"))
	}

	/// Offside rule: the lines indented (by tabs or spaces) below a line ending in `:` are its `{…}` block;
	/// None, with nothing consumed, when the next line is not indented deeper
	pub(super) fn parse_indented_block(&mut self) -> Option<Node> {
		let next_line = self.chars[self.pos..].iter().skip_while(|ch| **ch != '\n').skip(1);
		let indented_by_spaces = next_line.take_while(|ch| matches!(ch, ' ' | '\t')).any(|ch| *ch == ' ');
		let before_block = (self.pos, self.line_nr, self.column, self.current_line.clone());
		let outer_counts_spaces = self.indent_counts_spaces;
		self.indent_counts_spaces |= indented_by_spaces;
		let (_, indent) = self.skip_whitespace();
		if indent <= self.base_indent {
			self.indent_counts_spaces = outer_counts_spaces;
			(self.pos, self.line_nr, self.column, self.current_line) = before_block;
			return None;
		}
		let outer_indent = std::mem::replace(&mut self.base_indent, indent);
		let block = self.parse_list_with_separators(None, Bracket::None);
		self.base_indent = outer_indent;
		self.indent_counts_spaces = outer_counts_spaces;
		self.seek_back_over_whitespace(); // the newline after the block still separates the next statement
		Some(curly_block(block))
	}

	/// Ruby/Lua `do … end`, `then … end`, `else … end`: an `end` closes the statements opened here,
	/// counting the `do`/`then` openers in between; an `end` that closes nothing leaves the one-statement body
	pub(super) fn closing_end_follows(&self, openers: &[&str]) -> bool {
		self.closing_end_follows_from(self.pos, openers)
	}

	fn closing_end_follows_from(&self, start: usize, openers: &[&str]) -> bool {
		if self.names_end() {
			return false;
		}
		let mut open = 1;
		let mut position = start;
		while position < self.chars.len() {
			let ch = self.chars[position];
			if ch == '"' || ch == '\'' {
				position += 2 + self.chars[position + 1..].iter().position(|quoted| *quoted == ch).unwrap_or(self.chars.len());
				continue;
			}
			if !is_identifier_char(ch) {
				position += 1;
				continue;
			}
			let word_start = position;
			while position < self.chars.len() && is_identifier_char(self.chars[position]) {
				position += 1;
			}
			if word_start > 0 && self.chars[word_start - 1] == '.' {
				continue; // `range.end` is a property
			}
			let word: String = self.chars[word_start..position].iter().collect();
			match word.as_str() {
				END_KEYWORD => open -= 1,
				opener if openers.contains(&opener) => open += 1,
				_ => {}
			}
			if open == 0 {
				return true;
			}
		}
		false
	}

	/// Whether the source uses `end` as a name: a parameter or argument between separators (`f(a, end)`) or an assigned variable
	/// (`end = 3`). Then no `end` closes a block: `if a == 0 then 1 else end` hands back the variable
	fn names_end(&self) -> bool {
		let next_visible = |from: usize| self.chars[from..].iter().position(|ch| !ch.is_whitespace()).map(|offset| from + offset);
		let previous_visible = |before: usize| self.chars[..before].iter().rposition(|ch| !ch.is_whitespace());
		let mut position = 0;
		while position < self.chars.len() {
			let ch = self.chars[position];
			if ch == '"' || ch == '\'' {
				position += 2 + self.chars[position + 1..].iter().position(|quoted| *quoted == ch).unwrap_or(self.chars.len());
				continue;
			}
			if !is_identifier_char(ch) {
				position += 1;
				continue;
			}
			let word_start = position;
			while position < self.chars.len() && is_identifier_char(self.chars[position]) {
				position += 1;
			}
			if self.chars[word_start..position].iter().copied().ne(END_KEYWORD.chars()) {
				continue;
			}
			let before = previous_visible(word_start).map(|index| self.chars[index]);
			let after = next_visible(position);
			let assigned = after.is_some_and(|index| self.chars[index] == '=' && self.chars.get(index + 1) != Some(&'='));
			// `(…, end)` both sides: `foo(if c then 1 else 2 end)` closes a block
			if matches!(before, Some('(' | ',')) && after.is_some_and(|index| matches!(self.chars[index], ',' | ')')) || assigned {
				return true;
			}
		}
		false
	}

	/// The statements up to the closing `end` (consumed) as a `{…}` block; a `then` block also ends at its `else`
	pub(super) fn parse_end_block(&mut self, stops_at_else: bool) -> Node {
		let outer_else = std::mem::replace(&mut self.stops_at_else, stops_at_else);
		let outer_end = std::mem::replace(&mut self.stops_at_end, true);
		let outer_indent = self.base_indent;
		let before_first_line = (self.pos, self.line_nr, self.column, self.current_line.clone());
		let (_, indent) = self.skip_whitespace();
		(self.pos, self.line_nr, self.column, self.current_line) = before_first_line;
		self.base_indent = self.base_indent.max(indent); // its lines may be indented, the `end` line not
		let block = self.parse_list_with_separators(None, Bracket::None);
		self.base_indent = outer_indent;
		self.stops_at_else = outer_else;
		self.stops_at_end = outer_end;
		self.skip_whitespace();
		if self.matches_keyword(END_KEYWORD) {
			self.advance_by(END_KEYWORD.len());
		}
		curly_block(block)
	}

	/// Elixir's `fn a, b -> body end` at the cursor: the parameter names and where the body starts; Rust's
	/// `fn(x) -> i32 { … }` names a result type, no body
	fn elixir_function_head(&self) -> Option<(Vec<String>, usize)> {
		if !self.matches_keyword(ELIXIR_FUNCTION_KEYWORD) {
			return None;
		}
		let start = self.pos + ELIXIR_FUNCTION_KEYWORD.len();
		let line_end = (start..self.chars.len()).find(|&at| self.chars[at] == '\n').unwrap_or(self.chars.len());
		let arrow = (start..line_end.saturating_sub(1)).find(|&at| self.chars[at] == '-' && self.chars[at + 1] == '>')?;
		let head: String = self.chars[start..arrow].iter().collect();
		let is_parameter_text = head.chars().all(|ch| is_identifier_char(ch) || matches!(ch, ' ' | ',' | '(' | ')'));
		let names: Vec<String> = head.split([',', '(', ')', ' ']).filter(|name| !name.is_empty()).map(str::to_string).collect();
		let body_start = arrow + 2;
		let after: String = self.chars[body_start..].iter().collect();
		let mut words = after.trim_start().splitn(2, |ch: char| !is_identifier_char(ch));
		let is_result_type = words.next().is_some_and(|word| !word.is_empty()) && words.next().is_some_and(|rest| rest.trim_start().starts_with('{'));
		let is_function = is_parameter_text && !is_result_type && self.closing_end_follows_from(body_start, &[ELIXIR_FUNCTION_KEYWORD, "do"]);
		is_function.then_some((names, body_start))
	}

	/// `fn x -> x * 2 end`: the lambda `x => {x * 2}`
	pub(super) fn parse_elixir_function(&mut self) -> Option<Node> {
		let (names, body_start) = self.elixir_function_head()?;
		self.advance_by(body_start - self.pos);
		let body = self.parse_end_block(false);
		let parameters = match names.as_slice() {
			[single] => Node::Symbol(single.clone()),
			several => Node::List(several.iter().cloned().map(Node::Symbol).collect(), Bracket::Round, Separator::Colon),
		};
		Some(Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body)))
	}

	/// Python's `lambda a, b=2: a + b`, `lambda *xs: …`, `lambda: 42` at the cursor: the parameters as one group (the
	/// commas would split them into list items), then the body up to the end of the expression
	pub(super) fn parse_python_lambda(&mut self) -> Option<Node> {
		if !self.matches_keyword(PYTHON_LAMBDA_KEYWORD) || self.options.data_mode {
			return None;
		}
		let start = self.pos + PYTHON_LAMBDA_KEYWORD.len();
		let rest = &self.chars[start..];
		let colon = rest.iter().position(|ch| matches!(ch, ':' | '\n' | ';' | '{' | '[' | ')'))?;
		let parameters: String = rest[..colon].iter().collect();
		let names_a_variable = parameters.trim_start().starts_with('=');
		if rest[colon] != ':' || names_a_variable || rest.get(colon + 1) == Some(&'=') {
			return None;
		}
		let parameters = match parameters.trim() {
			"" => Empty,
			written => match parse(&format!("({written})")) {
				Node::Error(_) => return None,
				group => group,
			},
		};
		self.advance_by(start + colon + 1 - self.pos);
		self.skip_spaces();
		let body = self.parse_expr(Op::Assign.binding_power().1);
		let body = self.continue_expr(body, 0);
		Some(Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body)))
	}

	/// `🌍`, `🇩🇪`, `👍🏽` in code, one user-perceived character: one code point is a codepoint, several a text
	fn parse_emoji(&mut self) -> Node {
		let ahead: String = self.chars[self.pos..].iter().take(LONGEST_EMOJI).collect();
		let emoji = crate::extensions::strings::grapheme_clusters(&ahead)[0].to_string();
		self.advance_by(emoji.chars().count());
		let mut code_points = emoji.chars();
		match (code_points.next(), code_points.next()) {
			(Some(single), None) => Node::Char(single),
			_ => Node::Text(emoji),
		}
	}

	/// MATLAB's anonymous function `@(x) x.^2`: the lambda `x => x^2`
	fn parse_matlab_lambda(&mut self) -> Node {
		self.advance(); // '@'
		let parameters = self.parse_bracketed('(');
		self.skip_spaces();
		let body = self.parse_expr(Op::Assign.binding_power().1);
		Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body))
	}

	/// `\x ->`, `\a b ->`: names and an arrow after the backslash (`\alpha` alone is an entity)
	fn backslash_lambda_ahead(&self) -> bool {
		let rest: String = self.chars[self.pos + 1..].iter().take_while(|ch| **ch != '\n').collect();
		let Some((names, _)) = rest.split_once("->") else { return false };
		let names: Vec<&str> = names.split_whitespace().collect();
		!names.is_empty() && names.iter().all(|name| name.starts_with(char::is_alphabetic) && name.chars().all(|ch| ch.is_alphanumeric() || ch == '_'))
	}

	/// Haskell's `\x -> x * 2`, `\a b -> a + b`: the lambda
	fn parse_backslash_lambda(&mut self) -> Node {
		self.advance(); // '\\'
		let mut names = Vec::new();
		loop {
			self.skip_spaces();
			if self.current_char() == '-' && self.peek_char(1) == '>' {
				self.advance_by(2);
				break;
			}
			match self.at_identifier_start().then(|| self.parse_symbol().ok()).flatten() {
				Some(name) => names.push(Symbol(name)),
				None => return error("a lambda `\\x -> …` needs its arrow"),
			}
		}
		self.skip_spaces();
		let parameters = match names.len() {
			1 => names.remove(0),
			_ => Node::List(names, Bracket::Round, Separator::Colon),
		};
		let body = self.parse_expr(Op::Assign.binding_power().1);
		Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body))
	}

	/// `<T, F: Fn(i32) -> i32>` glued to a function's name and followed by its parameters: its length
	fn generic_parameters_length(&self) -> Option<usize> {
		if self.current_char() != '<' {
			return None;
		}
		let mut depth = 0;
		for (offset, &ch) in self.chars[self.pos..].iter().enumerate() {
			match ch {
				'<' => depth += 1,
				'>' if offset > 0 && self.chars[self.pos + offset - 1] == '-' => {} // the arrow of `Fn(i32) -> i32`
				'>' => {
					depth -= 1;
					if depth == 0 {
						return (self.peek_char(offset + 1) == '(').then_some(offset + 1);
					}
				}
				'\n' | ';' | '{' => return None,
				_ => {}
			}
		}
		None
	}

	/// Ruby's stabby lambda `->(x) { x * x }`, `-> { 42 }`: the lambda `x => {x * x}`
	fn parse_stabby_lambda(&mut self) -> Node {
		self.advance_by(2);
		self.skip_spaces();
		let parameters = match self.current_char() {
			'(' => self.parse_bracketed('('),
			_ => Empty,
		};
		self.skip_spaces();
		if self.current_char() != '{' {
			return error("a stabby lambda `->(x) { … }` needs its body in braces");
		}
		let body = self.parse_bracketed('{');
		Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body))
	}

	/// The `end` (or, in a `then` block, the `else`) that closes the block being parsed
	pub(super) fn at_block_close(&self) -> bool {
		self.stops_at_end && (self.matches_keyword(END_KEYWORD) || (self.stops_at_else && self.matches_keyword(ELSE_KEYWORD)))
	}

	/// `*rest`, `...rest`: the name after the `prefix` characters, starred
	fn parse_starred(&mut self, prefix: usize) -> Node {
		(0..prefix).for_each(|_| self.advance());
		match self.parse_symbol() {
			Ok(name) => Node::Symbol(format!("{}{name}", crate::tuples::STARRED)),
			Err(message) => error(&message),
		}
	}

	/// ` name(`: blanks, a name and its parenthesis
	fn named_call_follows(&self) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let name = (blanks..).take_while(|&offset| is_identifier_char(self.peek_char(offset))).count();
		blanks > 0 && name > 0 && self.is_identifier_start(blanks) && self.peek_char(blanks + name) == '('
	}

	/// ` () {` after a name
	fn empty_parameters_and_block_follow(&self) -> bool {
		let blanks_from = |start: usize| (start..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let blanks = blanks_from(0);
		let inner = blanks_from(blanks + 1);
		let closed = blanks + 1 + inner;
		blanks > 0 && self.peek_char(blanks) == '(' && self.peek_char(closed) == ')' && self.peek_char(closed + 1 + blanks_from(closed + 1)) == '{'
	}

	pub(super) fn parameters_follow_after_blanks(&self) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		blanks > 0 && self.peek_char(blanks) == '('
	}

	/// Parse symbol with optional suffix: name{...}, name<...>, name(...)
	/// Does NOT handle infix operators like : or = (those are handled by parse_expr)
	/// The soft keyword `version` before digits takes them as written: `version 1.10` is one value, not the float 1.1
	pub(super) fn version_operand(&mut self, symbol: &str) -> Option<Node> {
		if symbol != crate::versions::VERSION_KEYWORD || self.options.data_mode {
			return None;
		}
		let blanks = self.chars[self.pos..].iter().take_while(|c| **c == ' ').count();
		let length = crate::versions::operand_len(&self.chars[self.pos + blanks..]);
		if blanks == 0 || length == 0 {
			return None;
		}
		let operand: String = self.chars[self.pos + blanks..self.pos + blanks + length].iter().collect();
		self.advance_by(blanks + length);
		Some(Node::List(vec![Node::Symbol(symbol.to_string()), Node::Symbol(operand)], Bracket::None, Separator::Space))
	}

	pub(super) fn parse_symbol_with_suffix(&mut self) -> Node {
		let version_len = crate::versions::tagged_literal_len(&self.chars[self.pos..]);
		if version_len > 0 {
			let literal: String = self.chars[self.pos..self.pos + version_len].iter().collect();
			self.advance_by(version_len);
			return Node::Symbol(literal);
		}
		let symbol = match self.parse_symbol() {
			Ok(s) => s,
			Err(e) => return error(&e),
		};

		if let Some(version) = self.version_operand(&symbol) {
			return version;
		}

		// `def square (n) {…}` names its function like `def square(n) {…}`
		let names_function = std::mem::replace(&mut self.after_function_keyword, is_function_keyword(&symbol) && !self.options.wit_mode && !self.options.data_mode);
		// C's `int boom () { -1 }` (wiki/type.md): a name, `()` and a block are a definition like `boom() {…}`
		let spaced_definition = !self.options.wit_mode && !self.options.data_mode && self.empty_parameters_and_block_follow();
		if (names_function && self.parameters_follow_after_blanks()) || spaced_definition {
			self.skip_spaces();
		}
		// Rust's and C#'s `fn apply<F: Fn(i32) -> i32>(f: F)`, `id<T>(x: T)`: the type parameters say nothing warp needs
		// P157 (user): no generics in warp; the name carries the type parameters' names (GENERIC_MARK), welcome_forms.rs
		// drops their annotations: `fn id<T>(x: T) -> T` is `def id(x)`
		if let Some(length) = if names_function { self.generic_parameters_length() } else { None } {
			let written: String = self.chars[self.pos + 1..self.pos + length - 1].iter().collect();
			self.advance_by(length);
			self.generic_names = Some((symbol.clone(), type_parameter_names(&written)));
		}

		if let Some(reference) = self.function_reference_after(&symbol) {
			return reference;
		}

		if self.url_follows(&symbol) {
			return self.parse_url(symbol);
		}

		// C's `void f() { … }`: the result type of a function, not ø (src/lowering/declarations.rs c_function)
		if symbol == VOID_WORD && self.named_call_follows() {
			return Node::Symbol(symbol);
		}

		if symbol == NONE_WORD && self.peek_char(0) == '(' && !self.options.data_mode {
			return error(NONE_CALL_ERROR);
		}
		if let Some(constant) = check_constants(&symbol, self.options.data_mode).filter(|_| !self.at_member_name(symbol.chars().count())) {
			if !self.names_field(&symbol, &constant) {
				return self.refuse_constant_assignment(&symbol).unwrap_or(constant); // if true {} fall through :?
			}
		}

		if let Some(declaration) = self.try_parse_operator_declaration(&symbol) {
			return declaration;
		}

		// Possessive: `p's name` is the field `p.name`
		if !self.options.data_mode && self.current_char() == '\'' && self.peek_char(1) == 's' && self.peek_char(2) == ' ' && self.is_identifier_start(3) {
			self.advance_by(3);
			return match self.parse_symbol() {
				Ok(field) => Node::Key(Box::new(Symbol(symbol)), Op::Dot, Box::new(Symbol(field))),
				Err(message) => error(&message),
			};
		}

		// Optional type: `x:int?=ø`, `f(x:int?)`, the field `right? }` (wiki/null.md); a ternary `?` is followed by its branch instead
		if self.current_char() == '?' && (self.ends_optional_type(self.peek_char(1), self.peek_char(2)) || self.closes_after_blanks(1) || self.next_field_follows(1)) {
			self.advance();
			return Symbol(format!("{symbol}?"));
		}

		if symbol == HEX_WORD && !self.options.data_mode {
			if let Some(number) = self.try_parse_hex_word() {
				return number;
			}
		}

		// `To square a number: …` starts an English sentence with a capital
		if (symbol == TO_WORD || symbol == TO_SENTENCE_WORD) && !self.options.data_mode && !self.options.wit_mode && self.word_starts_statement(symbol.len()) {
			if let Some(definition) = self.try_parse_to_definition() {
				return definition;
			}
		}

		// Handle "global" keyword: global name = value
		if symbol == "for" {
			if let Some(loop_node) = self.try_parse_for_in() {
				return loop_node;
			}
		}

		if self.word_starts_statement(symbol.len()) {
			if let Some(skipped) = self.skip_foreign_modifiers(&symbol) {
				return skipped;
			}
		}

		if symbol == "global" || symbol == "export" {
			if let Some(declaration) = self.try_parse_global_declaration(&symbol) {
				return declaration;
			}
		}

		if symbol == NONLOCAL_WORD && self.word_starts_statement(symbol.len()) && self.name_after_blanks() {
			return self.parse_nonlocal_declaration();
		}

		// Handle "class"/"struct"/"type" keyword: class Name { fields }
		// But NOT type(x) which is a function call for type introspection
		if self.options.wit_mode && self.current_char() == '<' {
			return self.parse_type_application(symbol);
		}
		if !self.options.wit_mode && self.declares_type(&symbol) {
			return self.parse_type_declaration();
		}
		// `new Point(1, 2)` (Java, JavaScript, C#) of a declared class: the construction `Point(1, 2)`
		if symbol == NEW_WORD && !self.options.data_mode && self.declared_type_after_blanks() {
			while matches!(self.current_char(), ' ' | '\t') {
				self.advance();
			}
			let construction = self.parse_atom();
			let class = crate::lowering::class_methods::leading_name(&construction);
			crate::diagnostic::note_alias(&format!("{NEW_WORD} {class}"), &class);
			return construction;
		}
		if symbol == CONSTANT_ALIAS && !self.options.data_mode && self.identifier_after_blanks() {
			crate::diagnostic::note_alias(CONSTANT_ALIAS, CONST_WORD);
			return Symbol(CONST_WORD.to_string());
		}
		// Ruby's `attr_accessor :x, :y` in a class body: the fields x and y
		if RUBY_FIELD_WORDS.contains(&symbol.as_str()) && self.type_fields.is_some() {
			let line: String = (0..).map(|offset| self.peek_char(offset)).take_while(|ch| !matches!(ch, '\n' | '\0' | ';' | '}')).collect();
			let names: Vec<String> = line.split(',').map(|name| name.trim().trim_start_matches(':').to_string()).collect();
			if names.iter().all(|name| !name.is_empty() && name.chars().all(is_identifier_char)) {
				self.advance_by(line.chars().count());
				crate::diagnostic::note_alias(&format!("{symbol}{line}"), &names.join("; "));
				return Node::List(names.into_iter().map(Symbol).collect(), Bracket::None, Separator::Semicolon);
			}
		}
		if symbol == OPERATOR_WORD && !self.options.data_mode && matches!(self.current_char(), ' ' | '\t') {
			while matches!(self.current_char(), ' ' | '\t') {
				self.advance();
			}
			if let Some(head) = self.try_parse_operator_method_head() {
				crate::diagnostic::note_alias(&format!("{OPERATOR_WORD} {}", head.first().name()), &head.first().name());
				return head;
			}
		}
		// P178: `enum Shape { Circle(r), Rect(w, h), Dot }`, cases with values: the sum type `Circle(r) | Rect(w, h) | Dot`
		if symbol == ENUM_WORD && !self.options.wit_mode && !self.options.data_mode && self.enum_with_values_ahead() {
			return self.parse_enum_with_values();
		}
		// Kotlin's `enum class Color {…}`: the enum
		if symbol == ENUM_WORD && !self.options.wit_mode && self.class_keyword_after_blanks() == Some("class") {
			while matches!(self.current_char(), ' ' | '\t') {
				self.advance();
			}
			self.advance_by("class".len());
			crate::diagnostic::note_alias(&format!("{ENUM_WORD} class"), ENUM_WORD);
			return Symbol(symbol);
		}
		// `data class P(…)` (Kotlin), `open class`, `abstract class`: a modifier of a class declaration, the class itself
		if !self.options.wit_mode && CLASS_MODIFIERS.contains(&symbol.as_str()) {
			if let Some(keyword) = self.class_keyword_after_blanks() {
				crate::diagnostic::note_alias(&format!("{symbol} {keyword}"), keyword);
				while matches!(self.current_char(), ' ' | '\t') {
					self.advance();
				}
				self.advance_by(keyword.len());
				return self.parse_type_declaration();
			}
		}
		if !self.options.wit_mode && symbol == MIXIN_WORD && self.name_and_block_follow() {
			return match self.parse_type_declaration() {
				Node::Type { name, body } => Node::Type { name: Box::new(name.with_attribute(MIXIN_WORD, Node::True)), body },
				other => other,
			};
		}

		// `point {x:1}` with blanks constructs a declared type like the glued `point{x:1}` (open decision 41)
		// a capitalized word of no declared type before a block of fields is the tagged object `Person:{…}` (data, D4), as
		// glued `Person{…}` is: the README's `Person { name: "Alice" … }` (card person-name)
		let tagged = symbol.starts_with(|first: char| first.is_uppercase());
		let declared = self.declared_types.contains(&symbol);
		// inside a data literal any spaced `c { … }` is the child c{ … } (card spaced-child), but in a for header
		// `for t in todos { … }` it is the collection and the body (card markup-ul)
		let spaced_child = self.in_data_literal && !self.in_for_header && !declared && self.blanks_then('{');
		if spaced_child || ((declared || tagged) && self.block_after_blanks(declared)) {
			while matches!(self.current_char(), ' ' | '\t') {
				self.advance();
			}
			return self.parse_glued_suffix(symbol);
		}

		self.parse_glued_suffix(symbol)
	}

	/// `class circle{pi = 3}`, `class C{pi:int}`: a named number declared in a type body names the type's own field, which
	/// shadows nothing outside; the type's methods read the field
	fn names_field(&mut self, symbol: &str, constant: &Node) -> bool {
		if !matches!(constant, Node::Number(_)) {
			return false;
		}
		let declares = self.assignment_follows() || self.type_annotation_follows();
		let Some(fields) = self.type_fields.as_mut() else { return false };
		if declares {
			fields.insert(symbol.to_string());
		}
		fields.contains(symbol)
	}

	/// `:type` right after the name (`pi:int`, `pi: int`), not the definition `:=` nor a ternary's `c ? pi : 0`
	fn type_annotation_follows(&self) -> bool {
		self.current_char() == ':' && self.peek_char(1) != '='
	}

	/// `pi = 4` (P130, user: "loud error, if it was declared constant before which it should be"): a named number is a
	/// declared constant, an assignment to it is an error
	fn refuse_constant_assignment(&self, symbol: &str) -> Option<Node> {
		let column = self.column.saturating_sub(symbol.chars().count());
		let message = format!("{symbol} is a constant");
		self.assignment_follows().then(|| Diagnostic { message, line: self.line_nr, column, ..Default::default() }.fix("another name").into_error())
	}

	/// `= …` or `:= …` after blanks, not the comparison `==` nor the arrow `=>`
	fn assignment_follows(&self) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let defines = self.peek_char(blanks) == ':' && self.peek_char(blanks + 1) == '=';
		let assigns = self.peek_char(blanks) == '=' && !matches!(self.peek_char(blanks + 1), '=' | '>'); // `None => 0` is a match arm
		defines || assigns
	}

	/// `http://…`, `file://…`: the scheme of a URL, the rest of which reads as one text
	pub(super) fn url_follows(&self, symbol: &str) -> bool {
		URL_SCHEMES.contains(&symbol) && self.current_char() == ':' && self.peek_char(1) == '/' && self.peek_char(2) == '/'
	}

	pub(super) fn parse_url(&mut self, scheme: String) -> Node {
		let mut url = scheme;
		while !self.is_url_terminator(self.current_char()) {
			url.push(self.current_char());
			self.advance();
		}
		Node::Text(url)
	}

	/// `class Name {…}`, `struct`, `type Name {…}`, `record Name {…}`; `class:"btn"`, `type:email` (html attributes),
	/// `class="btn"`, `type = "text"` (html attributes, P188), `x.class`, `type(x)` and `type 3.5` are no declarations
	pub(super) fn declares_type(&self, symbol: &str) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let is_assigned = self.peek_char(blanks) == '=' && self.peek_char(blanks + 1) != '=';
		let is_key = (self.current_char() == ':' && self.peek_char(1) != '=') || is_assigned;
		let start = self.pos.saturating_sub(symbol.chars().count());
		let is_field = start > 0 && self.chars[start - 1] == '.';
		!is_key && !is_field && (TYPE_DECLARATION_WORDS.contains(&symbol)
			|| (symbol == RECORD_WORD && (self.name_and_block_follow() || self.name_and_parameters_follow()))
			// `type 3.5`, `type pi`, `type(x)`: the type of a value; a declaration names its type
			|| (symbol == "type" && self.type_declaration_follows()))
	}

	/// After `type`: a capitalized name (`type Point`) or any name with its body (`type point {…}`, `type size = u32`)
	fn type_declaration_follows(&self) -> bool {
		if !self.name_after_blanks() {
			return false;
		}
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let name_length = (blanks..).take_while(|&offset| is_identifier_char(self.peek_char(offset)) || self.peek_char(offset) == '-').count();
		let after_name = (blanks + name_length..).find(|&offset| !matches!(self.peek_char(offset), ' ' | '\t')).unwrap_or(blanks + name_length);
		let body_follows = TYPE_BODY_STARTS.contains(&self.peek_char(after_name)) && self.peek_char(after_name + 1) != '=';
		// `type of x` arrives as the declaration of `of`, which lowering (type_tests::type_of_word) reads as `type(x)`
		let of_follows = (blanks..blanks + name_length).map(|offset| self.peek_char(offset)).eq("of".chars());
		self.peek_char(blanks).is_uppercase() || body_follows || of_follows
	}

	/// The name and the `{fields}` of a type declaration, after its keyword
	pub(super) fn parse_type_declaration(&mut self) -> Node {
		let outer = self.type_fields.replace(Default::default());
		let declaration = self.parse_type_declaration_body();
		self.type_fields = outer;
		declaration
	}

	fn parse_type_declaration_body(&mut self) -> Node {
		self.skip_whitespace();
		let type_name = match self.parse_symbol() {
			Ok(name) => name,
			Err(message) => return error(&message),
		};
		// `class Box<T>{item:T}`: a field or parameter of a type parameter holds any value
		let type_parameters = match self.current_char() {
			// P157: warp has no generic syntax, a ported `class Box<T>` compiles untyped, with a note
			'<' => match self.parse_type_parameters() {
				Ok(parameters) => {
					crate::normalize::hint(&format!("{type_name}<{}>", parameters.join(", ")), &type_name, "warp infers types: write the class without type parameters");
					parameters
				}
				Err(message) => return error(&message),
			},
			// `type Option[T] = Some(T) | None` (samples/types.warp): warp's own spelling, no note
			'[' => match self.parse_type_parameters() {
				Ok(parameters) => parameters,
				Err(message) => return error(&message),
			},
			_ => vec![],
		};
		if let Some(variants_start) = self.variants_ahead() {
			return self.parse_sum_type(type_name, &type_parameters, variants_start);
		}
		let mut constructor_fields = if self.current_char() == '(' { self.parse_primary_constructor() } else { vec![] };
		let (kotlin_parent, claims) = self.skip_conformances().unwrap_or_default();
		// Python's `class Dog(Animal):` names its parents in the parentheses, `object` the root of all
		let python_body = self.current_char() == ':' && self.peek_char(1) != '=';
		let python_parent = match python_body {
			true => std::mem::take(&mut constructor_fields).into_iter().map(|parent| parent.drop_meta().name()).find(|parent| parent != PYTHON_ROOT_CLASS),
			false => None,
		};
		let ends_statement = matches!(self.peek_char((0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count()), '\n' | ';' | '}' | '\0');
		let before_body = (self.pos, self.line_nr, self.column, self.current_line.clone());
		// `class P(val x: Int)` ends at its line when no body follows on it
		match constructor_fields.is_empty() {
			true => {
				self.skip_whitespace();
			}
			false => while matches!(self.current_char(), ' ' | '\t') {
				self.advance();
			},
		}
		// `class dog extends animal {…}` (P117): the parent rides on the name, class_methods copies its fields and methods
		let mut name = Symbol(type_name);
		if let Some(parent) = python_parent.or(kotlin_parent) {
			name = name.with_attribute(EXTENDS_KEYWORD, Symbol(parent));
		}
		// P177: `implements Shape`, Swift's `: Shape` claim the traits, checked like `class Square{…} is Shape`
		if !claims.is_empty() {
			name = name.with_attribute(IMPLEMENTS_WORD, Node::List(claims, Bracket::None, Separator::Space));
		}
		if self.matches_keyword(EXTENDS_KEYWORD) {
			self.advance_by(EXTENDS_KEYWORD.len());
			self.skip_whitespace();
			match self.parse_symbol() {
				Ok(parent) => name = name.with_attribute(EXTENDS_KEYWORD, Symbol(parent)),
				Err(message) => return error(&message),
			}
			self.skip_whitespace();
		}
		// `class Duck with Walker, Swimmer {…}`: the mixins ride on the name, class_methods takes in their items
		if self.matches_keyword(WITH_KEYWORD) {
			self.advance_by(WITH_KEYWORD.len());
			let mut mixins = vec![];
			loop {
				self.skip_whitespace();
				match self.parse_symbol() {
					Ok(mixin) => mixins.push(Symbol(mixin)),
					Err(message) => return error(&message),
				}
				self.skip_whitespace();
				if self.current_char() != ',' {
					break;
				}
				self.advance();
			}
			name = name.with_attribute(WITH_KEYWORD, Node::List(mixins, Bracket::None, Separator::Space));
		}
		// Go's `type Shape interface {…}`: the trait Shape, as `interface Shape {…}` declares it (traits.rs)
		if self.matches_keyword(GO_INTERFACE_WORD) {
			self.advance_by(GO_INTERFACE_WORD.len());
			self.skip_whitespace();
			if self.current_char() == '{' {
				let block = self.parse_bracketed('{');
				return Node::List(vec![Symbol(GO_INTERFACE_WORD.to_string()), name, block], Bracket::None, Separator::Space);
			}
		}
		self.skip_struct_words();
		let body = if self.current_char() == '{' { Self::class_body(self.parse_bracketed('{')) } else { Empty };
		// Python's `class Point:` and the lines indented below it
		let body = match (python_body, body) {
			(true, Empty) => {
				self.advance(); // :
				self.skip_spaces();
				self.skip_struct_words();
				// `type Point: {` and its fields on the lines below: the braces are the body, as in `type Point {`
				match self.current_char() == '{' {
					true => Self::class_body(self.parse_bracketed('{')),
					false => self.parse_indented_block().map(Self::transform_fields_to_types).unwrap_or(Empty),
				}
			}
			// Ruby's `class Point` and the lines indented below it, up to its `end`
			(false, Empty) if self.pos > before_body.0 && self.closing_end_follows(&RUBY_END_OPENERS) => {
				(self.pos, self.line_nr, self.column, self.current_line) = before_body;
				let body = self.parse_indented_block().map(without_end_lines).map(Self::transform_fields_to_types).unwrap_or(Empty);
				self.skip_whitespace();
				if self.matches_keyword(END_KEYWORD) {
					self.advance_by(END_KEYWORD.len());
				}
				body
			}
			(_, body) => body,
		};
		let body = match constructor_fields.is_empty() {
			true => body,
			false => {
				let items = statements(body);
				let fields = constructor_fields.into_iter().map(Self::transform_fields_to_types);
				Node::List(fields.chain(items).collect(), Bracket::Curly, Separator::Semicolon)
			}
		};
		let body = if type_parameters.is_empty() { body } else { any_for_type_parameters(body, &type_parameters) };
		// Kotlin's `sealed class Shape`, Swift's `class Marker`: a class without fields, when its line ends after the name
		// (`type of x` goes on: no declaration)
		let bodyless = matches!(body, Empty) && ends_statement;
		let body = if bodyless { Node::List(vec![], Bracket::Curly, Separator::None) } else { body };
		// C#'s file-scoped `class Foo;` takes in the definitions after it (file_declarations.rs)
		if bodyless && self.current_char() == ';' {
			name = name.with_attribute(crate::lowering::file_declarations::FILE_CLASS_MARK, Node::True);
		}
		Node::Type { name: Box::new(name), body: Box::new(body) }
	}

	/// Go's `type Point struct {…}`, WebAssembly's `type Node: gc struct {…}`: the words before the fields
	fn skip_struct_words(&mut self) {
		for word in [GC_WORD, GO_STRUCT_WORD] {
			if self.matches_keyword(word) {
				self.advance_by(word.len());
				self.skip_spaces();
			}
		}
	}

	/// The traits a class names before its body, Java's and TypeScript's `implements Shape, Named {`, Swift's
	/// `struct Square: Shape {`, Kotlin's `class Square(…) : Shape {`: the parent and the claimed traits. A type conforms
	/// to a trait by defining its operations (notes/traits.md T2); a named one is a claim, checked (P177). Kotlin's
	/// superclass `: Shape()` (its constructor called) is the parent, as `extends Shape`
	fn skip_conformances(&mut self) -> Option<(Option<String>, Vec<Node>)> {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let start = if self.peek_char(blanks) == ':' && self.peek_char(blanks + 1) != '=' {
			blanks + 1
		} else if (0..IMPLEMENTS_WORD.len()).all(|i| self.peek_char(blanks + i) == IMPLEMENTS_WORD.as_bytes()[i] as char) && !is_identifier_char(self.peek_char(blanks + IMPLEMENTS_WORD.len())) {
			blanks + IMPLEMENTS_WORD.len()
		} else {
			return None;
		};
		let names: String = (start..).map(|offset| self.peek_char(offset)).take_while(|ch| !matches!(ch, '{' | '\n' | '\0')).collect();
		let names_list: Vec<&str> = names.split(',').map(str::trim).collect();
		let is_name = |name: &str| !name.is_empty() && name.chars().all(is_identifier_char);
		let parent = names_list.iter().find_map(|name| name.strip_suffix("()").filter(|parent| is_name(parent)));
		let are_names = names_list.iter().all(|name| is_name(name.strip_suffix("()").unwrap_or(name)));
		let ends_well = self.peek_char(start + names.chars().count()) == '{' || parent.is_some();
		if !are_names || !ends_well {
			return None;
		}
		let parent = parent.map(str::to_string);
		let claims = names_list.iter().filter(|name| !name.ends_with("()")).map(|name| Symbol(name.to_string())).collect();
		self.advance_by(start + names.trim_end().chars().count());
		Some((parent, claims))
	}

	/// The name of a declared type after blanks: `new Point(…)`
	fn declared_type_after_blanks(&self) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let name: String = (blanks..).map(|offset| self.peek_char(offset)).take_while(|ch| is_identifier_char(*ch)).collect();
		blanks > 0 && self.declared_types.contains(&name)
	}

	/// `class` or `struct` after blanks, the word a class modifier stands before
	fn class_keyword_after_blanks(&self) -> Option<&'static str> {
		let blanks = self.blanks_ahead();
		TYPE_DECLARATION_WORDS.into_iter().find(|keyword| {
			keyword.chars().enumerate().all(|(i, c)| self.peek_char(blanks + i) == c) && !is_identifier_char(self.peek_char(blanks + keyword.len()))
		})
	}

	fn blanks_ahead(&self) -> usize {
		(0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count()
	}

	/// `constant x`: a name after one or more blanks
	fn identifier_after_blanks(&self) -> bool {
		let blanks = self.blanks_ahead();
		blanks > 0 && (self.peek_char(blanks).is_alphabetic() || self.peek_char(blanks) == '_')
	}

	/// Kotlin's primary constructor `class Point(val x: Int, var y: Int = 0)`: its parameters are the fields
	fn parse_primary_constructor(&mut self) -> Vec<Node> {
		let parameters = statements(self.parse_bracketed('('));
		parameters.into_iter().map(|parameter| match parameter.drop_meta() {
			// `val x: Int`, `var y: Int = 0`: the field, its keyword dropped
			Node::List(words, _, _) if words.len() == 2 && FIELD_KEYWORDS.contains(&words[0].drop_meta().name().as_str()) => {
				let field = crate::lowering::class_methods::leading_name(&words[1]);
				crate::diagnostic::note_alias(&format!("{} {field}", words[0].drop_meta().name()), &field);
				words[1].clone()
			}
			// C#'s `int X`: the field X of type int
			Node::List(words, _, _) if words.len() == 2 && matches!(words[1].drop_meta(), Node::Symbol(_)) && crate::analyzer::type_word_kind(&words[0].drop_meta().name()).is_some() => {
				Node::Key(Box::new(words[1].clone()), Op::Colon, Box::new(words[0].clone()))
			}
			_ => parameter,
		}).collect()
	}

	/// `= Some(T) | None`, `: red | rgb(int, int, int)`, Elm's `= Failure HttpError | …` on lines of their own: the
	/// offset after the `=` or `:` when the declaration lists alternatives one of which carries a payload
	fn variants_ahead(&self) -> Option<usize> {
		let blanks = (0..).take_while(|&offset| self.peek_char(offset).is_whitespace()).count();
		let (sign, next) = (self.peek_char(blanks), self.peek_char(blanks + 1));
		let opens = (sign == '=' && !matches!(next, '=' | '>')) || (sign == ':' && next != '=');
		if !opens {
			return None;
		}
		let start = blanks + 1;
		let (mut depth, mut alternatives, mut payload, mut words) = (0, false, false, 0);
		let mut offset = start;
		loop {
			let ch = self.peek_char(offset);
			let after_word = is_identifier_char(self.peek_char(offset - 1));
			match ch {
				'\0' | ';' if depth == 0 => break,
				'\n' if depth == 0 => {
					let blanks = (offset + 1..).take_while(|&at| self.peek_char(at).is_whitespace()).count();
					if self.peek_char(offset + 1 + blanks) != '|' {
						break;
					}
				}
				'(' if depth == 0 && after_word => {
					payload = true;
					depth += 1;
				}
				'(' | '[' | '{' => depth += 1,
				')' | ']' | '}' if depth == 0 => break,
				')' | ']' | '}' => depth -= 1,
				'|' if depth == 0 && self.peek_char(offset + 1) == '|' => offset += 1,
				'|' if depth == 0 => (alternatives, words) = (true, 0),
				_ if depth == 0 && is_identifier_char(ch) && !after_word => {
					words += 1;
					payload |= words > 1;
				}
				_ => {}
			}
			offset += 1;
		}
		(alternatives && payload).then_some(start)
	}

	/// The sum type `Name = A(T) | B | C(x: int)` (card sum-types): the class Name without fields and a class extending
	/// it for each variant with a payload, its fields in order (an unnamed one is `value`, several `value1`, `value2`…);
	/// a variant without payload is its name, a symbol (`None` stays ø)
	fn parse_sum_type(&mut self, type_name: String, type_parameters: &[String], start: usize) -> Node {
		self.advance_by(start);
		let mut declarations = vec![];
		let mut bare_variants = vec![];
		loop {
			self.skip_spaces();
			let variant = match self.parse_symbol() {
				Ok(name) => name,
				Err(message) => return error(&format!("a variant of {type_name} needs a name: {message}")),
			};
			let mut payload = if self.current_char() == '(' { statements(self.parse_bracketed('(')) } else { vec![] };
			// Elm's `Failure HttpError`: the payload's types after the name
			self.skip_spaces();
			while self.is_identifier_start(0) {
				payload.extend(self.parse_symbol().ok().map(Symbol));
				self.skip_spaces();
			}
			match payload.is_empty() {
				true => bare_variants.push(Symbol(variant)),
				false => declarations.push(self.variant_class(&type_name, variant, payload, type_parameters)),
			}
			let blanks = (0..).take_while(|&offset| self.peek_char(offset).is_whitespace()).count();
			if self.peek_char(blanks) != '|' {
				return sum_type(type_name, bare_variants, declarations);
			}
			self.advance_by(blanks + 1);
		}
	}

	/// ` Shape { Circle(r), … }` after `enum`: a case followed by its values in parentheses
	fn enum_with_values_ahead(&self) -> bool {
		let mut offset = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let name_length = (offset..).take_while(|&at| is_identifier_char(self.peek_char(at))).count();
		if name_length == 0 {
			return false;
		}
		offset += name_length;
		offset += (offset..).take_while(|&at| self.peek_char(at).is_whitespace()).count();
		if self.peek_char(offset) != '{' {
			return false;
		}
		let mut depth = 0;
		loop {
			match self.peek_char(offset) {
				'\0' => return false,
				'(' if depth == 1 && is_identifier_char(self.peek_char(offset - 1)) => return true,
				'(' | '[' | '{' => depth += 1,
				')' | ']' | '}' if depth == 1 => return false,
				')' | ']' | '}' => depth -= 1,
				_ => {}
			}
			offset += 1;
		}
	}

	/// `enum Shape { Circle(r), Rect(w, h), Dot }`, Swift's `case circle(radius: Double)` lines: each case with values a
	/// class extending Shape, the others its variants by name, as the sum type `Circle(r) | Rect(w, h) | Dot`
	fn parse_enum_with_values(&mut self) -> Node {
		self.skip_spaces();
		let type_name = match self.parse_symbol() {
			Ok(name) => name,
			Err(message) => return error(&message),
		};
		self.skip_whitespace();
		let Node::List(cases, _, _) = self.parse_bracketed('{').drop_meta().clone() else { return error(&format!("enum {type_name} needs its cases in braces")) };
		let mut bare_variants = vec![];
		let mut declarations = vec![];
		for case in cases.iter().map(without_case_keyword) {
			match case.drop_meta() {
				Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => {
					declarations.push(self.variant_class(&type_name, items[0].drop_meta().name(), items[1..].to_vec(), &[]));
				}
				Node::Symbol(_) => bare_variants.push(case.drop_meta().clone()),
				other => return error(&format!("a case of enum {type_name} is a name with optional values, got {}", other.serialize())),
			}
		}
		sum_type(type_name, bare_variants, declarations)
	}

	fn variant_class(&mut self, type_name: &str, variant: String, payload: Vec<Node>, type_parameters: &[String]) -> Node {
		let count = payload.len();
		let field_name = |index: usize| if count == 1 { VARIANT_FIELD.to_string() } else { format!("{VARIANT_FIELD}{}", index + 1) };
		// `(T)` and `(radius: int)` arrive as groups of one
		let sole = |field: Node| match field.drop_meta() {
			Node::List(items, _, _) if items.len() == 1 => items[0].clone(),
			_ => field,
		};
		// P179: `rgb(r, g, b)` names its fields; `Some(T)`, `rgb(int, int, int)` and `Pair(Point, Point)` give types
		let is_type = |name: &str| crate::analyzer::builtin_type_kind(name).is_some() || type_parameters.iter().any(|parameter| parameter == name)
			|| self.declared_types.contains(name) || name.starts_with(char::is_uppercase);
		let mut untyped = type_parameters.to_vec();
		let fields: Vec<Node> = payload.into_iter().map(sole).enumerate().map(|(index, field)| match field.drop_meta() {
			Node::Key(_, Op::Colon, _) => field,
			Node::Symbol(name) if !is_type(name) => {
				untyped.push(name.clone());
				Node::Key(Box::new(Symbol(name.clone())), Op::Colon, Box::new(field))
			}
			_ => Node::Key(Box::new(Symbol(field_name(index))), Op::Colon, Box::new(field)),
		}).collect();
		let body = Self::transform_fields_to_types(Node::List(fields, Bracket::Curly, Separator::Semicolon));
		self.declared_types.insert(variant.clone());
		let name = Symbol(variant).with_attribute(EXTENDS_KEYWORD, Symbol(type_name.to_string()));
		Node::Type { name: Box::new(name), body: Box::new(any_for_type_parameters(body, &untyped)) }
	}

	/// `<T>`, `<A, B>`, `[T]`: the names of the type parameters of a declared type
	fn parse_type_parameters(&mut self) -> Result<Vec<String>, String> {
		let close = if self.current_char() == '[' { ']' } else { '>' };
		self.advance(); // < or [
		let mut names = vec![];
		loop {
			self.skip_whitespace();
			names.push(self.parse_symbol()?);
			self.skip_whitespace();
			match self.current_char() {
				',' => self.advance(),
				ch if ch == close => {
					self.advance();
					return Ok(names);
				}
				other => return Err(format!("type parameters <{}…> end with {close}, got {other}", names.join(", "))),
			}
		}
	}

	/// What is glued to a word: `name{…}`, `List<int>`, `p@unit`, `f(args)`, `f(params) {body}`; else the word itself
	pub(super) fn parse_glued_suffix(&mut self, symbol: String) -> Node {
		let ch = self.current_char();
		match ch {
			'{' => {
				// `point{x:1}` of a declared type constructs a point, `point:{x:1}` and any other `name{…}` stay data (D4)
				let op = if self.declared_types.contains(&symbol) { Op::None } else { Op::Colon };
				let outer_data_literal = std::mem::replace(&mut self.in_data_literal, op == Op::Colon);
				let outer_style_sheet = std::mem::replace(&mut self.in_style_sheet, op == Op::Colon && symbol == STYLE_WORD);
				self.tag_block = op == Op::Colon;
				let mut blocks = vec![self.parse_bracketed('{')];
				// `a{x:1}{y:2}{3}`: glued blocks are the children of a, `a{x}{y z}` is no `a{x, {y z}}`
				while self.current_char() == '{' {
					blocks.push(self.parse_bracketed('{'));
				}
				self.in_data_literal = outer_data_literal;
				self.in_style_sheet = outer_style_sheet;
				let block = match blocks.len() {
					1 => blocks.remove(0),
					_ => Node::List(blocks, Bracket::None, Separator::None),
				};
				Node::Key(Box::new(Symbol(symbol)), op, Box::new(block))
			}
			'<' if !self.options.xml_mode && !self.options.data_mode && self.type_application_length().is_some() => {
				let length = self.type_application_length().expect("guarded");
				let arguments: String = (1..length - 1).map(|offset| self.peek_char(offset)).collect();
				if symbol == "list" && !arguments.contains(',') {
					let element_words: Vec<&str> = arguments.split(|c: char| c == '<' || c == '>' || c.is_whitespace()).filter(|word| !word.is_empty()).collect();
					set_hint_position(self.line_nr, self.column.saturating_sub(symbol.chars().count()));
					norm::list_type(ListTypeStyle::Generic, &element_words);
				}
				self.advance_by(length);
				// `Box<int>(3)`, `Box<int>{item:3}` of a declared class `Box<T>`: its type parameters hold any value
				if self.declared_types.contains(&symbol) {
					return self.parse_glued_suffix(symbol);
				}
				Symbol(type_application_name(&symbol, &arguments))
			}
			'<' if !self.options.xml_mode && !self.peek_char(1).is_numeric() && self.peek_char(1) != '<' && names_a_type(&symbol) => {
				// Only treat as generic if immediately after a type name (no space)
				// and NOT followed by a number (that would be comparison: i<9); `it<k` and `a<b` compare
				let generic = self.parse_bracketed('<');
				Node::Key(Box::new(Symbol(symbol)), Op::Colon, Box::new(generic))
			}
			// `info@pannous.com` in data is one word; `p@unit` in code reads the meta key: `p.@unit`
			'@' if self.peek_char(1).is_alphabetic() && self.options.data_mode => {
				let rest: String = self.chars[self.pos..].iter().take_while(|c| is_identifier_char(**c) || matches!(c, '@' | '.' | '-')).collect();
				let rest = rest.trim_end_matches(['.', '-']).to_string();
				self.advance_by(rest.chars().count());
				Symbol(format!("{symbol}{rest}"))
			}
			'@' if self.peek_char(1).is_alphabetic() => {
				self.advance();
				match self.parse_symbol() {
					Ok(key) => Node::Key(Box::new(Symbol(symbol)), Op::Dot, Box::new(Symbol(format!("{ATTRIBUTE_MARK}{key}")))),
					Err(message) => error(&message),
				}
			}
			'(' => {
				// Parse arguments as a proper Node
				let args_node = self.parse_bracketed('(');
				// `add(2)(5)`: each glued group calls what the call before returned (closures.rs), one operand
				if self.current_char() == '(' && symbol != PRINT_WORD {
					let mut chain = vec![function_call(symbol, args_node)];
					while self.current_char() == '(' {
						chain.push(argument_group(self.parse_bracketed('(')));
					}
					return Node::List(chain, Bracket::None, Separator::Space);
				}
				self.skip_spaces(); // Only spaces, preserve newlines as statement separators

				// in a condition `if f(1, 2) {…}` the block is the body of the `if`, not of a definition of f
				if self.current_char() == '{' && !self.equals_compares && !self.in_for_header {
					// Function with body: name(params) { body }
					let body = self.parse_bracketed('{');
					let signature = Node::List(
						vec![Symbol(symbol), typed_parameters(args_node)],
						Bracket::Round,
						Separator::None,
					);
					Node::List(vec![signature, body], Bracket::Round, Separator::None)
				} else if symbol == PRINT_WORD {
					print_call(print_arguments(args_node))
				} else {
					function_call(symbol, args_node)
				}
			}
			_ => Node::symbol(&symbol),
		}
	}

	/// Helper to advance by N characters
	pub(super) fn advance_by(&mut self, n: usize) {
		for _ in 0..n {
			self.advance();
		}
	}
}

/// Function call: name(params) -> List([symbol, args...])
fn function_call(symbol: String, args_node: Node) -> Node {
	let mut items = vec![Symbol(symbol)];
	match typed_parameters(args_node) {
		Node::List(args, _, _) => items.extend(args),
		Node::Empty => {}
		other => items.push(other),
	}
	Node::List(items, Bracket::Round, Separator::None)
}

/// The arguments of a glued group `(5)`, `(1, 2)`, `()`, as a round list
fn argument_group(arguments: Node) -> Node {
	match arguments {
		Node::List(items, Bracket::Round, separator) => Node::List(items, Bracket::Round, separator),
		Node::List(items, _, _) => Node::List(items, Bracket::Round, Separator::None),
		Node::Empty => Node::List(vec![], Bracket::Round, Separator::None),
		other => Node::List(vec![other], Bracket::Round, Separator::None),
	}
}

/// A class body with each type parameter used as a type read as any type: a field `item:T` holds any value, a
/// parameter `with(x:T)` takes any
fn any_for_type_parameters(node: Node, parameters: &[String]) -> Node {
	let is_parameter = |node: &Node| match node.drop_meta() {
		Node::Symbol(name) => parameters.contains(name),
		Node::Type { name, body } => matches!(body.drop_meta(), Node::Empty) && parameters.contains(&name.drop_meta().name()),
		_ => false,
	};
	match node {
		Node::Key(name, Op::Colon, kind) if is_parameter(&kind) => Node::Key(name, Op::Colon, Box::new(Symbol(crate::type_kinds::UNTYPED_FIELD.to_string()))),
		Node::Key(head, Op::Define, body) => Node::Key(Box::new(untyped_parameters(*head, &is_parameter)), Op::Define, body),
		other => other.map_children(|child| any_for_type_parameters(child, parameters)),
	}
}

/// A method head with each parameter of a type parameter untyped: `with(x:T)` is `with(x)`
fn untyped_parameters(head: Node, is_parameter: &dyn Fn(&Node) -> bool) -> Node {
	match head {
		Node::Key(name, Op::Colon, kind) if is_parameter(&kind) => *name,
		Node::Meta { node, data } => Node::Meta { node: Box::new(untyped_parameters(*node, is_parameter)), data },
		other => other.map_children(|child| untyped_parameters(child, is_parameter)),
	}
}

/// The items of a block or parameter list `{a; b}`, `(a, b)`; a block of one statement `{fun sum() = x}` is that one
/// The sum type's declarations: the type itself, its variants without payload stay symbols, members of the type by name
/// (`red is Color`, P179), then the classes of the variants with payload
fn sum_type(type_name: String, bare_variants: Vec<Node>, declarations: Vec<Node>) -> Node {
	let name = Symbol(type_name).with_attribute(crate::lowering::sum_variants::VARIANTS_MARK, Node::List(bare_variants, Bracket::None, Separator::Space));
	let sum_type = Node::Type { name: Box::new(name), body: Box::new(Node::List(vec![], Bracket::Curly, Separator::None)) };
	Node::List([vec![sum_type], declarations].concat(), Bracket::None, Separator::Semicolon)
}

/// Swift's `case circle(radius: Double)`: the case without its keyword
fn without_case_keyword(case: &Node) -> Node {
	match case.drop_meta() {
		Node::List(words, _, _) if words.len() == 2 && words[0].drop_meta().name() == CASE_KEYWORD => words[1].clone(),
		_ => case.clone(),
	}
}

fn statements(block: Node) -> Vec<Node> {
	match block {
		Node::List(items, _, Separator::Colon | Separator::Semicolon | Separator::Newline) => items,
		Empty => vec![],
		Node::List(items, Bracket::Curly | Bracket::Round, separator) => vec![Node::List(items, Bracket::None, separator)],
		single => vec![single],
	}
}

/// A Ruby class body without the `end` lines of its methods (their bodies are read by indentation)
fn without_end_lines(body: Node) -> Node {
	let is_end = |item: &Node| matches!(item.drop_meta(), Node::Symbol(word) if word == END_KEYWORD);
	match body {
		Node::List(items, bracket, separator) => Node::List(items.into_iter().filter(|item| !is_end(item)).map(without_end_lines).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(without_end_lines(*left)), op, Box::new(without_end_lines(*right))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(without_end_lines(*node)), data },
		other => other,
	}
}
