//! Paths and URLs: where a path literal starts and a URL ends
use super::*;

impl WarpParser {
	/// `&name`: a reference to the function `name`, an `&` glued to the word after it and not to a word before it (`a &b`, `f(&g)`)
	/// `./mozart.mp3`, `../songs/a.wav`: `./` or `../` glued to what follows and not to an operand before it starts a
	/// file path without quotes; `xs ./ 2` and `xs./2` divide each element
	pub(crate) fn starts_path_literal(&self) -> bool {
		let dots = if self.peek_char(1) == '.' { 2 } else { 1 };
		let glued_to_operand =
			is_identifier_char(self.prev_char()) || matches!(self.prev_char(), ')' | ']' | '}' | '"' | '\'');
		self.current_char() == '.'
			&& self.peek_char(dots) == '/'
			&& !ends_path(self.peek_char(dots + 1))
			&& !glued_to_operand
	}

	/// Check if character terminates a URL
	pub(crate) fn is_url_terminator(&self, ch: char) -> bool {
		ch == '\0'
			|| ch == ' '
			|| ch == '\t'
			|| ch == '\n'
			|| ch == '\r'
			|| ch == ';'
			|| ch == ')'
			|| ch == ']'
			|| ch == '}'
			|| ch == '>'
			|| ch == '"'
			|| ch == '\''
			|| ch == ','
			|| ch == '«'
	}

	/// The path literal at the cursor, up to whitespace, a closing bracket or a separator
	pub(crate) fn parse_path_literal(&mut self) -> Node {
		let path: String =
			(0..).map(|offset| self.peek_char(offset)).take_while(|&c| !super::lookahead::ends_path(c)).collect();
		self.advance_by(path.chars().count());
		Node::Text(path)
	}

	/// Parse symbol with optional suffix: name{...}, name<...>, name(...)
	/// Does NOT handle infix operators like : or = (those are handled by parse_expr)
	/// The soft keyword `version` before digits takes them as written: `version 1.10` is one value, not the float 1.1
	pub(crate) fn version_operand(&mut self, symbol: &str) -> Option<Node> {
		if symbol != crate::words::VERSION_KEYWORD || self.options.data_mode {
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

	/// `http://…`, `file://…`, `ssh://…`: the scheme of a URL, the rest of which reads as one text
	pub(crate) fn url_follows(&self) -> bool {
		URL_MARK.chars().enumerate().all(|(offset, mark)| self.peek_char(offset) == mark)
	}

	pub(crate) fn parse_url(&mut self, scheme: String) -> Node {
		let mut url = scheme;
		while !self.is_url_terminator(self.current_char()) {
			url.push(self.current_char());
			self.advance();
		}
		Node::Text(url)
	}
}
