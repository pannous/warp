//! The cursor over the source: characters, positions, keywords, whitespace and comments

use super::*;

impl WaspParser {
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

	/// The operator a block holds alone, the cursor at its start: `{++}`, `{ * }`, and its length up to the `}`
	pub(super) fn bare_operator_in_block(&self) -> Option<(String, usize)> {
		const OPERATOR_CHARS: &str = "+-*/%^<>=!&|";
		let is_blank = |ch: &char| matches!(ch, ' ' | '\t');
		let opened = self.chars[..self.pos].iter().rev().find(|ch| !is_blank(ch)) == Some(&'{');
		let rest = &self.chars[self.pos..];
		let operator: String = rest.iter().take_while(|ch| OPERATOR_CHARS.contains(**ch)).collect();
		let length = operator.chars().count() + rest[operator.chars().count()..].iter().take_while(|ch| is_blank(ch)).count();
		(opened && !operator.is_empty() && rest.get(length) == Some(&'}')).then_some((operator, length))
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

	/// The operand after a single `|`: a bare word or word operator is marked as a possible pipe stage, any other operand gets the hint
	/// toward `or`
	pub(super) fn pipe_operand(&self, operand: Node, line: usize, column: usize) -> Node {
		if crate::pipes::may_be_stage(&operand) {
			return crate::pipes::pipe_stage(operand);
		}
		set_hint_position(line, column);
		norm::operator(PIPE_GLYPH, false);
		operand
	}

	pub(super) fn get_position(&self) -> (usize, usize) {
		(self.line_nr, self.column)
	}

	/// The word of `length` characters just read (0: the one at the cursor) follows a `.`: `math.sqrt`, `math.pi` name
	/// a member, no operator or constant
	pub(super) fn at_member_name(&self, length: usize) -> bool {
		let start = self.pos.saturating_sub(length);
		start > 0 && self.chars.get(start - 1) == Some(&'.') && self.chars.get(start).is_some_and(|first| first.is_alphabetic())
	}

	pub(super) fn prev_char(&self) -> char {
		if self.pos == 0 { '\0' } else { *self.chars.get(self.pos - 1).unwrap_or(&'\0') }
	}

	/// Check if input at current position matches a keyword (followed by non-alphanumeric)
	pub(super) fn matches_keyword(&self, keyword: &str) -> bool {
		keyword.chars().enumerate().all(|(i, c)| self.peek_char(i) == c)
			&& !is_identifier_char(self.peek_char(keyword.len()))
	}

	/// `#` followed by a space (`# note`), a shebang `#!`, a doc comment `##` or a directive (`#use lib`) starts a comment;
	/// `#` directly followed by anything else counts (`#s`, `#a-1`, `#f(x)`)
	pub(super) fn at_hash_comment(&self) -> bool {
		if matches!(self.peek_char(1), ' ' | '\t' | '\n' | '\r' | '\0' | '!' | '#') {
			return true;
		}
		let word: String = (1..).map(|offset| self.peek_char(offset)).take_while(|&c| is_identifier_char(c)).collect();
		HASH_DIRECTIVES.contains(&word.as_str())
	}

	/// `//` right behind an operand (`7//2`, `x//=2`, `f(x)//2`) divides; after a space or `:` (URLs) it starts a comment
	pub(super) fn at_floor_division(&self) -> bool {
		let previous = self.prev_char();
		self.current_char() == '/' && self.peek_char(1) == '/' && self.pos > 0 && (is_identifier_char(previous) || matches!(previous, ')' | ']'))
	}

	/// `.+ .- .* ./` at the cursor: the element-wise form of the arithmetic operator
	pub(super) fn element_wise_operator(&self) -> Option<Op> {
		if self.current_char() != '.' {
			return None;
		}
		ELEMENT_WISE_OPERATORS.iter().find(|(glyph, _)| *glyph == self.peek_char(1)).map(|(_, op)| *op)
	}

	/// `1 -1`, `[1 +1]`: a number, a space, then a sign glued to a number (D13). Asked: the list `[1 -1]` (the default,
	/// a warning when unanswered) or the arithmetic `1 - 1`. Variables, calls and words keep subtracting: `x -1`
	pub(super) fn signed_number_starts_a_list(&mut self, lhs: &Node) -> Result<bool, Node> {
		if self.signed_list_element == Some(self.pos) {
			return Ok(true); // answered for the operand inside
		}
		if !matches!(lhs.drop_meta(), Node::Number(_)) || !self.prev_char().is_whitespace() || !self.number_starts_at(1) {
			return Ok(false);
		}
		let signed: String = self.chars[self.pos..].iter().take_while(|ch| !ch.is_whitespace() && !matches!(ch, ']' | ')' | '}' | ',' | ';')).collect();
		let (first, sign, number) = (lhs.serialize(), self.current_char(), &signed[1..]);
		let written = format!("{first} {signed}");
		let readings = vec![
			crate::diagnostic::reading("the list", &format!("{first} ({signed})")), // also inside brackets: `[1 (-1) 2]`
			crate::diagnostic::reading("arithmetic", &format!("{first} {sign} {number}")),
		];
		let question = crate::diagnostic::Ask::new(SIGNED_OPERAND_TOPIC, format!("is `{written}` a list or arithmetic?"), readings, crate::diagnostic::Fallback::Warning)
			.written(&written).at(self.line_nr, self.column);
		let is_list = crate::diagnostic::ask(&question)? == 0;
		if is_list {
			self.signed_list_element = Some(self.pos);
		}
		Ok(is_list)
	}

	/// R's `<-` at the cursor (P145): `x <- 3` assigns with a note to write `x = 3`; the cramped `x<-3` is an error
	/// naming `x = 3` and `x < -3`
	pub(super) fn left_arrow_assignment(&self, op: Op, target: &Node) -> Result<(), Node> {
		if op != Op::Assign || (self.current_char(), self.peek_char(1)) != ('<', '-') {
			return Ok(());
		}
		let ends_value = |ch: &char| ch.is_whitespace() || matches!(ch, ';' | '(' | ')' | ']' | '}');
		let rest: Vec<char> = self.chars[self.pos + 2..].iter().copied().skip_while(|ch| *ch == ' ').collect();
		let token: String = rest.iter().take_while(|ch| !ends_value(ch)).collect();
		let continues = rest.get(token.chars().count()).is_some_and(|ch| !matches!(ch, '\n' | ';' | ')' | ']' | '}'));
		let value = if continues { format!("{token} …") } else { token };
		let name = target.serialize();
		let assignment = format!("{name} = {value}");
		let is_spaced = self.peek_char(2).is_whitespace();
		let (written, readings, fallback) = match is_spaced {
			true => (format!("{name} <- {value}"), vec![reading("the assignment", &assignment)], Fallback::Warning),
			false => (format!("{name}<-{value}"), vec![reading("the assignment", &assignment), reading("the comparison", &format!("{name} < -{value}"))], Fallback::Error),
		};
		let forms: Vec<String> = readings.iter().map(|reading| reading.explicit_form.clone()).collect();
		let question = format!("`{written}`: write {}", forms.join(" or "));
		let question = Ask::new(LEFT_ARROW_TOPIC, question, readings, fallback).written(&written).at(self.line_nr, self.column);
		ask(&question).map(|_| ())
	}

	/// Code stands before the cursor on its line (`x = 7 // note`, not a `// note` line of its own)
	/// Code that could be divided ends right before: a name, a number, `)` or `]` (`xs[1] // 2`); after a text, a comma
	/// or a brace `//` is plainly a comment
	fn follows_operand_on_its_line(&self) -> bool {
		let last = self.chars[..self.pos].iter().rev().take_while(|&&c| c != '\n').find(|c| !c.is_whitespace());
		last.is_some_and(|&c| c.is_alphanumeric() || matches!(c, '_' | ')' | ']'))
	}

	/// The `//` at the cursor is followed by what a divisor could be: one word (`// 2`, `// n`) or an expression
	/// (`// n + 1`), not prose (`// property with value list`)
	fn comment_reads_like_divisor(&self) -> bool {
		let rest: String = self.chars[self.pos + 2..].iter().take_while(|&&c| c != '\n').collect();
		let rest = rest.trim();
		!rest.contains(char::is_whitespace) || rest.contains(['+', '-', '*', '/', '%', '(', '^'])
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
		let start = trimmed.rfind(|c: char| !is_identifier_char(c)).map_or(0, |at| at + trimmed[at..].chars().next().map_or(1, char::len_utf8));
		match &trimmed[start..] {
			"then" => "then",
			ELSE_KEYWORD => ELSE_KEYWORD,
			_ => "",
		}
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
			let starts_method = self.peek_char(length) == '.' && (self.peek_char(length + 1).is_alphabetic() || self.peek_char(length + 1) == '_');
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

	pub(super) fn consume_rest_of_line(&mut self) -> String {
		let mut line_comment = String::new();
		loop {
			let ch = self.current_char();
			if ch == '\0' {
				break;
			}
			if ch == '\n' {
				self.advance();
				break;
			}
			line_comment.push(ch);
			self.advance();
		}
		line_comment.trim().to_string()
	}

	/// `/* … */` and `/# … #/` open a block comment
	pub(super) fn at_block_comment_start(&self) -> bool {
		self.current_char() == '/' && matches!(self.peek_char(1), '*' | '#')
	}

	/// The text of a `/* … */` or `/# … #/` comment, the parser standing on its opening; comments nest:
	/// `/* outer /* inner */ still outer */`
	pub(super) fn consume_block_comment(&mut self) -> String {
		let mark = if self.peek_char(1) == '#' { '#' } else { '*' };
		self.advance_by(2);
		let mut block = String::new();
		let mut depth = 1;
		while self.current_char() != '\0' {
			let pair = (self.current_char(), self.peek_char(1));
			if pair == (mark, '/') || pair == ('/', mark) {
				depth += if pair.0 == '/' { 1 } else { -1 };
				if depth == 0 {
					self.advance_by(2);
					break;
				}
				block.extend([pair.0, pair.1]);
				self.advance_by(2);
				continue;
			}
			block.push(self.current_char());
			self.advance();
		}
		block
	}

	/// Spaces and the comments that may sit inside a line: `1 /* inline */ + 1` and `a b # note`; the newline stays a separator
	pub(super) fn skip_spaces_and_inline_comments(&mut self) {
		loop {
			self.skip_spaces();
			match (self.current_char(), self.peek_char(1)) {
				_ if self.at_block_comment_start() => { self.consume_block_comment(); }
				('#', ' ' | '\t') => {
					while !matches!(self.current_char(), '\n' | '\0') {
						self.advance();
					}
				}
				_ => return,
			}
		}
	}

	pub(super) fn skip_whitespace_and_comments(&mut self) -> (bool, usize, Option<String>) {
		let mut had_newline = false;
		let mut line_indent = 0;
		let mut comments = Vec::new();
		loop {
			let (newline, indent) = self.skip_whitespace();
			had_newline |= newline;
			if newline { line_indent = indent; }

			let (c1, c2) = (self.current_char(), self.peek_char(1));

			// # line comment (shell-style) at line start: `# note`, `#!`, `##`, `#use …`; `#x`, `#(…)`, `#a-1` count
			if c1 == '#' && self.is_at_line_start() && self.at_hash_comment() {
				self.advance();
				let text = self.consume_rest_of_line();
				if !text.is_empty() { comments.push(text); }
				had_newline = true;
				continue;
			}
			// // line comment (but not :// URL scheme)
			if c1 == '/' && c2 == '/' && self.prev_char() != ':' && !self.at_floor_division() {
				if self.follows_operand_on_its_line() && self.comment_reads_like_divisor() {
					self.set_hint_pos();
					crate::diagnostic::educate_once(SLASH_COMMENT_TOPIC, "a // b", "a//b", "`// …` after code is a comment; floor division is written glued: a//b");
				}
				self.advance_by(2);
				let text = self.consume_rest_of_line();
				if !text.is_empty() { comments.push(text); }
				had_newline = true;
				continue;
			}
			// /* block comment */ and /# block comment #/
			if self.at_block_comment_start() {
				let block = self.consume_block_comment();
				had_newline |= block.contains('\n');
				let trimmed = block.trim();
				if !trimmed.is_empty() { comments.push(trimmed.to_string()); }
				continue;
			}
			break;
		}
		let comment = if comments.is_empty() { None } else { Some(comments.join("\n")) };
		(had_newline, line_indent, comment)
	}

	/// Skip characters until the target character is found
	pub(super) fn skip_until(&mut self, target: char) {
		while !self.end_of_input() && self.current_char() != target {
			self.advance();
		}
	}
}
