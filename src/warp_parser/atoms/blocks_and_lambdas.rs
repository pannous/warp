//! Blocks and lambdas as other languages write them: definition bodies, the offside rule, `end` blocks (Ruby, Lua, Elixir), and the lambdas of Python, MATLAB, Haskell and Ruby
use super::*;

impl WarpParser {
	/// The rest of the line, or the indented lines below it, or a `{…}` block
	pub(in crate::warp_parser) fn parse_definition_body(&mut self) -> Node {
		if self.current_char() == '{' {
			return self.parse_atom();
		}
		if !self.only_blanks_before_newline() {
			return self.rest_of_statement(); // `to mail x to address: print address`
		}
		self.parse_indented_block()
			.unwrap_or_else(|| error("a definition needs a body: `to name params: body`"))
	}

	pub(in crate::warp_parser) fn next_line_starts_with_tab(&self) -> bool {
		self.chars[self.pos..].iter().skip_while(|ch| **ch != '\n').nth(1) == Some(&'\t')
	}

	/// The `end` on the line after a block read by indentation, the newline before the next statement kept; an `end`
	/// indented less than the block's head closes an outer block
	pub(in crate::warp_parser) fn skip_end_line(&mut self) {
		let before = self.mark();
		let (_, indent) = self.skip_whitespace();
		if indent != self.base_indent || !self.matches_keyword(END_KEYWORD) {
			self.rewind(before);
			return;
		}
		self.advance_by(END_KEYWORD.len());
	}

	/// Offside rule: the lines indented (by tabs or spaces) below a line ending in `:` are its `{…}` block;
	/// None, with nothing consumed, when the next line is not indented deeper
	pub(in crate::warp_parser) fn parse_indented_block(&mut self) -> Option<Node> {
		let next_line = self.chars[self.pos..].iter().skip_while(|ch| **ch != '\n').skip(1);
		let indented_by_spaces = next_line.take_while(|ch| matches!(ch, ' ' | '\t')).any(|ch| *ch == ' ');
		let before_block = self.mark();
		let outer_counts_spaces = self.indent_counts_spaces;
		self.indent_counts_spaces |= indented_by_spaces;
		let (_, indent) = self.skip_whitespace();
		if indent <= self.base_indent {
			self.indent_counts_spaces = outer_counts_spaces;
			self.rewind(before_block);
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
	pub(in crate::warp_parser) fn closing_end_follows(&self, openers: &[&str]) -> bool {
		self.closing_end_follows_from(self.pos, openers)
	}

	pub(super) fn closing_end_follows_from(&self, start: usize, openers: &[&str]) -> bool {
		if self.names_end() {
			return false;
		}
		let mut open = 1;
		for (word_start, word_end) in code_words(&self.chars, start) {
			if word_start > 0 && self.chars[word_start - 1] == '.' {
				continue; // `range.end` is a property
			}
			let word: String = self.chars[word_start..word_end].iter().collect();
			match word.as_str() {
				END_KEYWORD => open -= 1,
				opener if openers.contains(&opener) && !is_endless_definition(&self.chars, word_end) => open += 1,
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
	pub(super) fn names_end(&self) -> bool {
		let next_visible = |from: usize| {
			self.chars[from..]
				.iter()
				.position(|ch| !ch.is_whitespace())
				.map(|offset| from + offset)
		};
		let previous_visible = |before: usize| self.chars[..before].iter().rposition(|ch| !ch.is_whitespace());
		code_words(&self.chars, 0).any(|(word_start, word_end)| {
			if self.chars[word_start..word_end].iter().copied().ne(END_KEYWORD.chars()) {
				return false;
			}
			let before = previous_visible(word_start).map(|index| self.chars[index]);
			let after = next_visible(word_end);
			let assigned =
				after.is_some_and(|index| self.chars[index] == '=' && self.chars.get(index + 1) != Some(&'='));
			// `(…, end)` both sides: `foo(if c then 1 else 2 end)` closes a block
			matches!(before, Some('(' | ',')) && after.is_some_and(|index| matches!(self.chars[index], ',' | ')'))
				|| assigned
		})
	}

	/// The statements up to the closing `end` (consumed) as a `{…}` block; a `then` block also ends at its `else`
	pub(in crate::warp_parser) fn parse_end_block(&mut self, stops_at_else: bool) -> Node {
		let outer_else = std::mem::replace(&mut self.stops_at_else, stops_at_else);
		let outer_end = std::mem::replace(&mut self.stops_at_end, true);
		let outer_indent = self.base_indent;
		let before_first_line = self.mark();
		let (_, indent) = self.skip_whitespace();
		self.rewind(before_first_line);
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
	pub(super) fn elixir_function_head(&self) -> Option<(Vec<String>, usize)> {
		if !self.matches_keyword(ELIXIR_FUNCTION_KEYWORD) {
			return None;
		}
		let start = self.pos + ELIXIR_FUNCTION_KEYWORD.len();
		let line_end = (start..self.chars.len())
			.find(|&at| self.chars[at] == '\n')
			.unwrap_or(self.chars.len());
		let arrow =
			(start..line_end.saturating_sub(1)).find(|&at| self.chars[at] == '-' && self.chars[at + 1] == '>')?;
		let head: String = self.chars[start..arrow].iter().collect();
		let is_parameter_text = head
			.chars()
			.all(|ch| is_identifier_char(ch) || matches!(ch, ' ' | ',' | '(' | ')'));
		let names: Vec<String> = head
			.split([',', '(', ')', ' '])
			.filter(|name| !name.is_empty())
			.map(str::to_string)
			.collect();
		let body_start = arrow + 2;
		let after: String = self.chars[body_start..].iter().collect();
		let mut words = after.trim_start().splitn(2, |ch: char| !is_identifier_char(ch));
		let is_result_type = words.next().is_some_and(|word| !word.is_empty())
			&& words.next().is_some_and(|rest| rest.trim_start().starts_with('{'));
		let is_function = is_parameter_text
			&& !is_result_type
			&& self.closing_end_follows_from(body_start, &[ELIXIR_FUNCTION_KEYWORD, "do"]);
		is_function.then_some((names, body_start))
	}

	/// `fn x -> x * 2 end`: the lambda `x => {x * 2}`
	pub(in crate::warp_parser) fn parse_elixir_function(&mut self) -> Option<Node> {
		let (names, body_start) = self.elixir_function_head()?;
		self.advance_by(body_start - self.pos);
		let body = self.parse_end_block(false);
		let parameters = match names.as_slice() {
			[single] => Node::Symbol(single.clone()),
			several => Node::List(
				several.iter().cloned().map(Node::Symbol).collect(),
				Bracket::Round,
				Separator::Colon,
			),
		};
		Some(Node::Key(Box::new(parameters), Op::Arrow, Box::new(body)))
	}

	/// Python's `lambda a, b=2: a + b`, `lambda *xs: …`, `lambda: 42` at the cursor: the parameters as one group (the
	/// commas would split them into list items), then the body up to the end of the expression
	pub(in crate::warp_parser) fn parse_python_lambda(&mut self) -> Option<Node> {
		if !self.matches_keyword(PYTHON_LAMBDA_KEYWORD) || self.options.data_mode {
			return None;
		}
		let start = self.pos + PYTHON_LAMBDA_KEYWORD.len();
		let rest = &self.chars[start..];
		let colon = rest
			.iter()
			.position(|ch| matches!(ch, ':' | '\n' | ';' | '{' | '[' | ')'))?;
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
		Some(Node::Key(Box::new(parameters), Op::Arrow, Box::new(body)))
	}
}

impl WarpParser {
	/// MATLAB's anonymous function `@(x) x.^2`: the lambda `x => x^2`
	pub(super) fn parse_matlab_lambda(&mut self) -> Node {
		self.advance(); // '@'
		let parameters = self.parse_bracketed('(');
		self.skip_spaces();
		let body = self.parse_expr(Op::Assign.binding_power().1);
		Node::Key(Box::new(parameters), Op::Arrow, Box::new(body))
	}

	/// `\x ->`, `\a b ->`: names and an arrow after the backslash (`\alpha` alone is an entity)
	pub(super) fn backslash_lambda_ahead(&self) -> bool {
		let rest: String = self.chars[self.pos + 1..]
			.iter()
			.take_while(|ch| **ch != '\n')
			.collect();
		let Some((names, _)) = rest.split_once("->") else {
			return false;
		};
		let names: Vec<&str> = names.split_whitespace().collect();
		!names.is_empty()
			&& names.iter().all(|name| {
				name.starts_with(char::is_alphabetic) && name.chars().all(|ch| ch.is_alphanumeric() || ch == '_')
			})
	}

	/// Haskell's `\x -> x * 2`, `\a b -> a + b`: the lambda
	pub(super) fn parse_backslash_lambda(&mut self) -> Node {
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
		Node::Key(Box::new(parameters), Op::Arrow, Box::new(body))
	}

	/// `<T, F: Fn(i32) -> i32>` glued to a function's name and followed by its parameters: its length
	pub(super) fn generic_parameters_length(&self) -> Option<usize> {
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
	pub(super) fn parse_stabby_lambda(&mut self) -> Node {
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
		Node::Key(Box::new(parameters), Op::Arrow, Box::new(body))
	}

	/// The `end` (or, in a `then` block, the `else`) that closes the block being parsed
	pub(in crate::warp_parser) fn at_block_close(&self) -> bool {
		self.stops_at_end
			&& (self.matches_keyword(END_KEYWORD) || (self.stops_at_else && self.matches_keyword(ELSE_KEYWORD)))
	}
}

/// A Ruby class body without the `end` lines of its methods (their bodies are read by indentation)
pub(super) fn without_end_lines(body: Node) -> Node {
	let is_end = |item: &Node| item.is_symbol(END_KEYWORD);
	match body {
		Node::List(items, bracket, separator) => Node::List(
			items
				.into_iter()
				.filter(|item| !is_end(item))
				.map(without_end_lines)
				.collect(),
			bracket,
			separator,
		),
		other => children_rewritten(other, without_end_lines),
	}
}

/// The spans of the words of the code from `start` on: an `end` in a text or a comment is no word of the code
/// (`// … would end it`)
/// Ruby's endless method `def twice = x * 2`, warp's `def twice := x * 2` (after the `def` at `def_end`): its name,
/// parameters and `=` on one line, closed by no `end`
pub(super) fn is_endless_definition(chars: &[char], def_end: usize) -> bool {
	let line: String = chars[def_end..].iter().take_while(|&&ch| ch != '\n').collect();
	let head = line
		.trim_start()
		.trim_start_matches(|ch: char| is_identifier_char(ch) || ch == '?' || ch == '!')
		.trim_start();
	let after_parameters = match head.strip_prefix('(') {
		Some(parameters) => parameters.find(')').map_or("", |close| &parameters[close + 1..]),
		None => head,
	};
	let after_parameters = after_parameters.trim_start();
	after_parameters.starts_with(":=")
		|| (after_parameters.starts_with('=')
			&& !after_parameters.starts_with("==")
			&& !after_parameters.starts_with("=>"))
}

pub(super) fn code_words(chars: &[char], start: usize) -> impl Iterator<Item = (usize, usize)> + '_ {
	let mut position = start;
	std::iter::from_fn(move || {
		while position < chars.len() {
			if let Some(after) = text_or_comment_end(chars, position) {
				position = after;
			} else if is_identifier_char(chars[position]) {
				let word_start = position;
				while position < chars.len() && is_identifier_char(chars[position]) {
					position += 1;
				}
				return Some((word_start, position));
			} else {
				position += 1;
			}
		}
		None
	})
}
