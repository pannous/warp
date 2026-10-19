//! The cursor over the source: characters, positions, keywords, whitespace

use super::*;

/// Where the cursor stands, to go back to after a look ahead
#[derive(Clone)]
pub(super) struct Mark {
	pub(super) pos: usize,
	line_nr: usize,
	column: usize,
	current_line: String,
}

impl WarpParser {
	pub(super) fn mark(&self) -> Mark {
		Mark { pos: self.pos, line_nr: self.line_nr, column: self.column, current_line: self.current_line.clone() }
	}

	pub(super) fn rewind(&mut self, mark: Mark) {
		Mark { pos: self.pos, line_nr: self.line_nr, column: self.column, current_line: self.current_line } = mark;
	}

	/// The next `length` characters, consumed
	pub(super) fn take_chars(&mut self, length: usize) -> String {
		let taken = self.chars[self.pos..self.pos + length].iter().collect();
		self.advance_by(length);
		taken
	}

	pub(super) fn end_of_input(&self) -> bool {
		self.pos >= self.chars.len()
	}

	pub(super) fn current_char(&self) -> char {
		*self.chars.get(self.pos).unwrap_or(&'\0')
	}

	pub(super) fn is_identifier_start(&self, offset: usize) -> bool {
		let letter = self.peek_char(offset);
		letter.is_alphabetic() || letter == '_'
	}

	/// A bound name `$h` at the offset: a hole of a code pattern, or a reference
	pub(super) fn is_hole_start(&self, offset: usize) -> bool {
		self.peek_char(offset) == '$' && self.is_identifier_start(offset + 1)
	}

	pub(super) fn peek_char(&self, offset: usize) -> char {
		*self.chars.get(self.pos + offset).unwrap_or(&'\0')
	}

	/// Do blanks, an identifier, optional blanks and a `{` follow the cursor: `record point {…}`
	pub(super) fn name_and_block_follow(&self) -> bool {
		self.name_then('{')
	}

	/// ` Point(int X, int Y)`: a name with parameters, C#'s positional `record Point(…)`
	pub(super) fn name_and_parameters_follow(&self) -> bool {
		self.name_then('(')
	}

	fn name_then(&self, opener: char) -> bool {
		let is_blank = |ch: char| matches!(ch, ' ' | '\t');
		let mut offset = 0;
		while is_blank(self.peek_char(offset)) {
			offset += 1;
		}
		let name_start = offset;
		while is_identifier_char(self.peek_char(offset)) {
			offset += 1;
		}
		let has_name = offset > name_start;
		while is_blank(self.peek_char(offset)) {
			offset += 1;
		}
		has_name && self.peek_char(offset) == opener
	}

	pub(super) fn advance(&mut self) {
		let ch = self.current_char();
		self.char = ch; // debug
		if ch == '\n' {
			self.line_nr += 1;
			self.column = 1;
			self.current_line = self.lines.get(self.line_nr - 1).cloned().unwrap_or_default();
		} else {
			self.column += 1;
		}
		self.pos += 1;
	}

	pub(super) fn advance_by(&mut self, count: usize) {
		for _ in 0..count {
			self.advance();
		}
	}

	/// The characters from here on that `keep`, consumed
	pub(super) fn take_while_char(&mut self, keep: impl Fn(char) -> bool) -> String {
		let length = self.chars[self.pos..].iter().take_while(|&&ch| keep(ch)).count();
		self.take_chars(length)
	}

	/// `skip` characters, then a name given to `build`; a missing name is the error
	pub(super) fn symbol_after(&mut self, skip: usize, build: impl FnOnce(String) -> Node) -> Node {
		self.advance_by(skip);
		self.parse_symbol().map_or_else(|message| error(&message), build)
	}

	pub(super) fn get_position(&self) -> (usize, usize) {
		(self.line_nr, self.column)
	}

	/// The word of `length` characters just read (0: the one at the cursor) follows a `.`: `math.sqrt`, `math.pi` name
	/// a member, no operator or constant
	pub(super) fn at_member_name(&self, length: usize) -> bool {
		self.word_follows(length, &['.'])
	}

	/// The word of `length` characters just read follows a capitalized `Type::`: `Node::True` names a case of the enum
	/// Node, no constant (card node-enum); `f64::sqrt` keeps its operator word, `std::f64::consts::PI` its constant
	pub(super) fn at_scoped_name(&self, length: usize) -> bool {
		let scope_end = self.pos.saturating_sub(length + 2);
		let scope_start = self.chars[..scope_end].iter().rposition(|c| !is_identifier_char(*c)).map_or(0, |i| i + 1);
		self.word_follows(length, &[':', ':']) && self.chars.get(scope_start).is_some_and(|first| first.is_uppercase())
	}

	fn word_follows(&self, length: usize, prefix: &[char]) -> bool {
		let start = self.pos.saturating_sub(length);
		start >= prefix.len()
			&& self.chars[start - prefix.len()..start] == *prefix
			&& self.chars.get(start).is_some_and(|first| first.is_alphabetic())
	}

	pub(super) fn prev_char(&self) -> char {
		if self.pos == 0 {
			'\0'
		} else {
			*self.chars.get(self.pos - 1).unwrap_or(&'\0')
		}
	}

	/// Check if input at current position matches a keyword (followed by non-alphanumeric)
	pub(super) fn matches_keyword(&self, keyword: &str) -> bool {
		keyword.chars().enumerate().all(|(i, c)| self.peek_char(i) == c)
			&& !is_identifier_char(self.peek_char(keyword.len()))
	}

	pub(super) fn is_at_line_start(&self) -> bool {
		// Check if we're at the very beginning or right after whitespace/newline
		self.pos == 0 || self.prev_char().is_whitespace()
	}

	/// Skip whitespace, return (had_newline, current_line_indent)
	/// Only tabs count as semantic indent (not spaces)
	pub(super) fn skip_whitespace(&mut self) -> (bool, usize) {
		let mut had_newline = false;
		let mut line_indent = 0;
		loop {
			let ch = self.current_char();
			if !ch.is_whitespace() {
				break;
			}
			if ch == '\n' {
				had_newline = true;
				line_indent = 0; // reset for new line
			} else if ch == '\t' || (ch == ' ' && self.context.indent_counts_spaces) {
				line_indent += 1; // tabs count as indent, spaces only inside a space-indented block
			}
			self.advance();
		}
		(had_newline, line_indent)
	}

	/// Neither a data literal file nor a WIT file: warp code with its keywords
	pub(super) fn in_code(&self) -> bool {
		!self.options.wit_mode && !self.options.data_mode
	}

	/// Skips spaces and tabs, not line continuations
	pub(super) fn skip_blanks(&mut self) {
		while matches!(self.current_char(), ' ' | '\t') {
			self.advance();
		}
	}

	/// A tab between two items of a line separates columns: `"1+1"⇥2` is a list of two (tab-separated test pairs,
	/// decision warp-is-tests). The blanks around the cursor hold the tab; indentation and a tab before a comment
	/// or the line's end separate nothing.
	pub(super) fn at_tab_separator(&self) -> bool {
		let is_blank = |c: &&char| matches!(c, ' ' | '\t');
		let behind = self.chars[..self.pos].iter().rev().take_while(is_blank).count();
		let ahead = self.chars[self.pos..].iter().take_while(is_blank).count();
		let blanks = &self.chars[self.pos - behind..self.pos + ahead];
		let content_before = self.pos > behind && self.chars[self.pos - behind - 1] != '\n';
		let content_after = !matches!(
			(self.peek_char(ahead), self.peek_char(ahead + 1)),
			('\n' | '\r' | '\0', _) | ('/', '/' | '*') | ('#', ' ' | '\t' | '\n' | '\0')
		);
		blanks.contains(&'\t') && content_before && content_after
	}

	/// Blanks and line continuations: a `\` at the end of a line joins the next line to the statement
	pub(super) fn skip_spaces(&mut self) {
		loop {
			if self.current_char() == ' ' || self.current_char() == '\t' {
				self.advance();
			} else if let Some(continuation_length) = self.line_continuation_length() {
				self.advance_by(continuation_length);
			} else {
				return;
			}
		}
	}

	/// Is the keyword at the cursor followed (after blanks) by `{`: `div {…}` is a tag, not the operator
	pub(super) fn word_opens_block(&self, word: &str) -> bool {
		let after = word.chars().count();
		let gap = (after..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		self.peek_char(after + gap) == '{'
	}

	/// Does an operand follow, after blanks, the `width` characters at the cursor (not the line's end or a closing bracket)
	pub(super) fn operand_after(&self, width: usize) -> bool {
		let gap = (width..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		gap > 0 && !matches!(self.peek_char(width + gap), '\n' | '\r' | '\0' | ')' | ']' | '}' | ';' | ',')
	}

	/// The identifier starting `offset` characters ahead
	pub(super) fn word_at(&self, offset: usize) -> String {
		(offset..).map(|at| self.peek_char(at)).take_while(|c| is_identifier_char(*c)).collect()
	}

	/// The identifier that ends right before the cursor and the blanks before it, on this line
	pub(super) fn word_before(&self) -> &str {
		let line = &self.input_before_cursor();
		let trimmed = line.trim_end_matches([' ', '\t', '\r']);
		let start = trimmed
			.rfind(|c: char| !is_identifier_char(c))
			.map_or(0, |at| at + trimmed[at..].chars().next().map_or(1, char::len_utf8));
		match &trimmed[start..] {
			"then" => "then",
			ELSE_KEYWORD => ELSE_KEYWORD,
			_ => "",
		}
	}

	/// The identifier that ends right before position `end` and the blanks before it
	pub(super) fn word_ending_before(&self, end: usize) -> String {
		let before = self.chars[..end].iter().rev().skip_while(|c| matches!(c, ' ' | '\t'));
		let word: String = before.take_while(|c| is_identifier_char(**c)).collect();
		word.chars().rev().collect()
	}

	/// The source line up to the cursor
	pub(super) fn input_before_cursor(&self) -> String {
		let before: String = self.chars[..self.pos].iter().rev().take_while(|c| **c != '\n').collect();
		before.chars().rev().collect()
	}

	/// Length of `\` plus trailing blanks and the newline, when the backslash ends its line; or of the line break and
	/// indentation before a method call that starts the next line (`numbers\n    .map(square)`, as in JS, Kotlin, Swift)
	pub(super) fn line_continuation_length(&self) -> Option<usize> {
		if matches!(self.current_char(), '\n' | '\r') && !self.options.data_mode {
			let mut length = 1;
			while matches!(self.peek_char(length), ' ' | '\t' | '\r' | '\n') {
				length += 1;
			}
			let starts_method = self.peek_char(length) == '.'
				&& (self.peek_char(length + 1).is_alphabetic() || self.peek_char(length + 1) == '_');
			let starts_pipeline = self.peek_char(length) == '|' && self.peek_char(length + 1) == '>';
			// `if c then⏎ a⏎ else⏎ b`: a line ending in then/else, or one starting with else, continues the if
			let starts_else = self.word_at(length) == ELSE_KEYWORD;
			let ends_in_branch_word = matches!(self.word_before(), "then" | ELSE_KEYWORD);
			return (starts_method || starts_pipeline || starts_else || ends_in_branch_word).then_some(length);
		}
		if self.current_char() != '\\' {
			return None;
		}
		let mut length = 1;
		while matches!(self.peek_char(length), ' ' | '\t' | '\r') {
			length += 1;
		}
		(self.peek_char(length) == '\n').then_some(length + 1)
	}

	/// Skip characters until the target character is found
	pub(super) fn skip_until(&mut self, target: char) {
		while !self.end_of_input() && self.current_char() != target {
			self.advance();
		}
	}

	pub(super) fn at_identifier_start(&self) -> bool {
		self.current_char().is_alphabetic() || self.current_char() == '_'
	}

	pub(crate) fn blanks_ahead(&self) -> usize {
		(0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count()
	}
}

const MAX_DIVISOR_NAME_LENGTH: usize = 2;

pub(crate) fn is_number_or_short_name(word: &str) -> bool {
	let is_name = word.len() <= MAX_DIVISOR_NAME_LENGTH && word.chars().all(|c| c.is_ascii_lowercase() || c == '_');
	is_name || word.parse::<f64>().is_ok()
}

pub(crate) fn is_identifier_char(c: char) -> bool {
	c.is_alphanumeric() || c == '_' || is_music_accidental(c)
}

/// The sharp and flat signs continue a name: `C♯4`, `D♭5` are note names (lib/sound.warp)
pub(crate) fn is_music_accidental(c: char) -> bool {
	matches!(c, '♯' | '♭')
}
