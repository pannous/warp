//! Blocks: `{…}`, `do … end`, colon and indented bodies

use super::expressions::BODY_ENDS;
use super::*;
use crate::node::children_rewritten;
use crate::words::IN_WORD;

/// A loop body of one comma line holds code, one statement: `x, y = y, x + y` swaps, as `{a, b=2}` would be data
pub(super) fn statement_block(block: Node) -> Node {
	match block {
		Node::List(items, Bracket::Curly, Separator::Comma) => {
			Node::List(vec![Node::List(items, Bracket::None, Separator::Comma)], Bracket::Curly, Separator::Semicolon)
		}
		block => block,
	}
}

/// A loop body as a `{…}` block: `print i` becomes `{print i}`, a block stays itself
pub(super) fn as_block(body: Node) -> Node {
	match body.drop_meta() {
		Node::List(_, Bracket::Curly, _) => body,
		_ => Node::List(vec![body], Bracket::Curly, Separator::Semicolon),
	}
}

impl WarpParser {
	/// The body after `:` or `do`: the statements up to `end` after `do`, the indented block under a `:` or `do` at the end
	/// of the line, else the rest of the line
	pub(super) fn colon_body(&mut self, word: &str) -> Node {
		self.advance_by(word.len());
		if word == DO_WORD && self.closing_end_follows(&END_BLOCK_OPENERS) {
			return self.parse_end_block(false);
		}
		self.indented_lines_below().unwrap_or_else(|| self.rest_of_statement())
	}

	/// The lines indented below a head that ends its line without `:`, as below `for x in xs:` (`while m > 1` and its
	/// lines); None, with nothing consumed, for a head followed by anything else
	pub(super) fn indented_lines_below(&mut self) -> Option<Node> {
		if !self.only_blanks_before_newline() {
			return None;
		}
		self.with_equals_comparing(false, |parser| parser.parse_indented_block()).map(statement_block)
		// its lines assign
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
		let next_visible =
			|from: usize| self.chars[from..].iter().position(|ch| !ch.is_whitespace()).map(|offset| from + offset);
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

	/// The rest of the body after `if c:` or `while c:`, of which the colon took only the first word or words
	pub(super) fn continue_colon_body(&mut self, body: &Node) -> Node {
		match body.drop_meta() {
			_ if is_print_word(body) && self.braceless_argument_follows() => {
				// `if it%2: print it`, `if c: print ord x`
				let first = self.expression_of_words();
				self.print_call_from(first, Self::expression_of_words)
			}
			// `if c: print a, b`: the colon took `print a`, the comma continues its arguments
			Node::List(words, Bracket::None, Separator::Space)
				if words.len() > 1 && is_print_word(&words[0]) && self.current_char() == ',' =>
			{
				self.print_call_from(one_expression(&words[1..]), |parser| parser.parse_expr(0))
			}
			_ => self.continue_expr(body.clone(), 0),
		}
	}

	/// A statement body follows: not a separator, closing bracket, end of input or the `do` keyword
	pub(super) fn at_body_start(&self) -> bool {
		!BODY_ENDS.contains(&self.current_char()) && !self.matches_keyword("do")
	}

	/// Python's `try:` and `except:` end with a colon; the block may start on the next line
	pub(super) fn skip_python_colon(&mut self) {
		if self.current_char() == ':' {
			self.advance();
			self.skip_whitespace();
		}
	}

	/// The operator a block holds alone, the cursor at its start: `{++}`, `{ * }`, and its length up to the `}`
	pub(super) fn bare_operator_in_block(&self) -> Option<(String, usize)> {
		const OPERATOR_CHARS: &str = "+-*/%^<>=!&|";
		let is_blank = |ch: &char| matches!(ch, ' ' | '\t');
		let opened = self.chars[..self.pos].iter().rev().find(|ch| !is_blank(ch)) == Some(&'{');
		let rest = &self.chars[self.pos..];
		let operator: String = rest.iter().take_while(|ch| OPERATOR_CHARS.contains(**ch)).collect();
		let length =
			operator.chars().count() + rest[operator.chars().count()..].iter().take_while(|ch| is_blank(ch)).count();
		(opened && !operator.is_empty() && rest.get(length) == Some(&'}')).then_some((operator, length))
	}

	/// The rest of the line, or the indented lines below it, or a `{…}` block
	pub(in crate::warp_parser) fn parse_definition_body(&mut self) -> Node {
		if self.current_char() == '{' {
			return self.parse_atom();
		}
		if !self.only_blanks_before_newline() {
			return self.rest_of_statement(); // `to mail x to address: print address`
		}
		self.parse_indented_block().unwrap_or_else(|| error("a definition needs a body: `to name params: body`"))
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
			items.into_iter().filter(|item| !is_end(item)).map(without_end_lines).collect(),
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
	let head =
		line.trim_start().trim_start_matches(|ch: char| is_identifier_char(ch) || ch == '?' || ch == '!').trim_start();
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

/// `f(xs) := xs.map { it*10 }` is `f(xs) := (xs.map {it*10})`: the block belongs to the method call, not to the statement
pub(super) fn with_trailing_block(assignment: Node, block: Node) -> Node {
	match assignment {
		meta @ Node::Meta { .. } => meta.under_meta(|node| with_trailing_block(node, block)),
		Node::Key(target, op, value) if assigned_method_call(&value) => {
			Node::Key(target, op, Box::new(with_trailing_block(*value, block)))
		}
		Node::Key(target, op, value) => {
			Node::Key(target, op, Box::new(Node::List(vec![*value, block], Bracket::None, Separator::Space)))
		}
		other => other,
	}
}

/// The body block at the end of an `if`/`while` header: `c {b}` is the pair [c {b}], and in `i < n {b}` the
/// block juxtaposes only with the rightmost operand `n`, so it is split off the right spine of the comparison
pub(super) fn split_trailing_block(header: &Node, empty_is_block: bool) -> Option<(Node, Node)> {
	match header.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 => match items[1].drop_meta() {
			Node::List(_, Bracket::Curly, _) => Some((items[0].clone(), items[1].clone())),
			Empty if empty_is_block => Some((items[0].clone(), items[1].clone())),
			_ => None,
		},
		// `if x in xs {…}`: the block after the collection is the body
		Node::List(items, bracket, separator) if items.len() == 3 && items[1].is_symbol(IN_WORD) => {
			split_trailing_block(&items[2], empty_is_block).map(|(collection, block)| {
				(
					Node::List(
						vec![items[0].clone(), items[1].clone(), collection],
						bracket.clone(),
						separator.clone(),
					),
					block,
				)
			})
		}
		Node::Key(left, op, right) if op.is_comparison() || op.is_arithmetic() || op.is_logical() || op.is_prefix() => {
			split_trailing_block(right, empty_is_block)
				.map(|(operand, block)| (Node::Key(left.clone(), *op, Box::new(operand)), block))
		}
		_ => None,
	}
}

/// Statements as a `{…}` block body
pub(super) fn curly_block(statements: Node) -> Node {
	match statements {
		Node::List(items, Bracket::None, separator) => Node::List(items, Bracket::Curly, separator),
		single => Node::List(vec![single], Bracket::Curly, Separator::None),
	}
}
