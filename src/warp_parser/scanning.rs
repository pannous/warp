//! The cursor over the source: characters, positions, keywords, whitespace

use super::*;

/// Where the cursor stands, to go back to after a look ahead
#[derive(Clone)]
pub(super) struct Mark {
	pub(super) pos: usize,
	line_nr: usize,
	column: usize,
	current_line: String,
	atoms_read: usize,
}

impl WarpParser {
	pub(super) fn mark(&self) -> Mark {
		Mark {
			pos: self.pos,
			line_nr: self.line_nr,
			column: self.column,
			current_line: self.current_line.clone(),
			atoms_read: self.atoms.len(),
		}
	}

	pub(super) fn rewind(&mut self, mark: Mark) {
		let atoms_read;
		Mark {
			pos: self.pos,
			line_nr: self.line_nr,
			column: self.column,
			current_line: self.current_line,
			atoms_read,
		} = mark;
		self.atoms.truncate(atoms_read);
	}

	/// `read` (an atom's parser) with the atom it reads logged: kind, where it starts, the characters it took
	pub(super) fn reading_atom<T>(&mut self, kind: AtomKind, read: impl FnOnce(&mut Self) -> T) -> T {
		let (start, line, column) = (self.pos, self.line_nr, self.column);
		let value = read(self);
		let written = self.chars[start..self.pos].iter().collect();
		self.atoms.push(Atom {
			kind,
			line,
			column,
			written,
		});
		value
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
		let start = self.pos.saturating_sub(length);
		start > 0
			&& self.chars.get(start - 1) == Some(&'.')
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

	/// `1 -1`, `[1 +1]`: a number, a space, then a sign glued to a number (D13). Asked: the list `[1 -1]` (the default,
	/// a warning when unanswered) or the arithmetic `1 - 1`. Variables, calls and words keep subtracting: `x -1`.
	/// Inside a list literal a space and a glued sign start the next item after anything (decision list-sign):
	/// `a=5; [a -1]` is [5 -1], while `[a - 1]` and `[a-1]` subtract
	pub(super) fn signed_number_starts_a_list(&mut self, lhs: &Node) -> Result<bool, Node> {
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
			crate::diagnostic::reading("the list", &format!("{first} ({signed})")), // also inside brackets: `[1 (-1) 2]`
			crate::diagnostic::reading("arithmetic", &format!("{first} {sign} {number}")),
		];
		let question = crate::diagnostic::Ask::new(
			SIGNED_OPERAND_TOPIC,
			format!("is `{written}` a list or arithmetic?"),
			readings,
			crate::diagnostic::Fallback::Warning,
		)
		.written(&written)
		.at(self.line_nr, self.column);
		let is_list = crate::diagnostic::ask(&question)? == 0;
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
			} else if ch == '\t' || (ch == ' ' && self.indent_counts_spaces) {
				line_indent += 1; // tabs count as indent, spaces only inside a space-indented block
			}
			self.advance();
		}
		(had_newline, line_indent)
	}

	/// Blanks and line continuations: a `\` at the end of a line joins the next line to the statement
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
		let gap = (after..)
			.take_while(|at| matches!(self.peek_char(*at), ' ' | '\t'))
			.count();
		self.peek_char(after + gap) == '{'
	}

	/// Does an operand follow, after blanks, the `width` characters at the cursor (not the line's end or a closing bracket)
	pub(super) fn operand_after(&self, width: usize) -> bool {
		let gap = (width..)
			.take_while(|at| matches!(self.peek_char(*at), ' ' | '\t'))
			.count();
		gap > 0
			&& !matches!(
				self.peek_char(width + gap),
				'\n' | '\r' | '\0' | ')' | ']' | '}' | ';' | ','
			)
	}

	/// The identifier starting `offset` characters ahead
	pub(super) fn word_at(&self, offset: usize) -> String {
		(offset..)
			.map(|at| self.peek_char(at))
			.take_while(|c| is_identifier_char(*c))
			.collect()
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
		let before: String = self.chars[..self.pos]
			.iter()
			.rev()
			.take_while(|c| **c != '\n')
			.collect();
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
}

const MAX_DIVISOR_NAME_LENGTH: usize = 2;

pub(crate) fn is_number_or_short_name(word: &str) -> bool {
	let is_name = word.len() <= MAX_DIVISOR_NAME_LENGTH && word.chars().all(|c| c.is_ascii_lowercase() || c == '_');
	is_name || word.parse::<f64>().is_ok()
}
