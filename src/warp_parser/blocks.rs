//! Blocks: `{…}`, `do … end`, colon and indented bodies

use super::*;

/// A loop body of one comma line holds code, one statement: `x, y = y, x + y` swaps, as `{a, b=2}` would be data
fn statement_block(block: Node) -> Node {
	match block {
		Node::List(items, Bracket::Curly, Separator::Colon) => Node::List(
			vec![Node::List(items, Bracket::None, Separator::Colon)],
			Bracket::Curly,
			Separator::Semicolon,
		),
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
		self.with_equals_comparing(false, |parser| parser.parse_indented_block())
			.map(statement_block) // its lines assign
	}
}
