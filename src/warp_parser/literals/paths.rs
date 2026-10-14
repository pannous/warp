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
}
