//! Comments: `//`, `#` and `/* */` comments, trailing comments; a `//` meant as floor division warns, naming `/!`
use super::scanning::is_number_or_short_name;
use super::*;

/// `(col // 3) * 3`: a spaced `//` after code starts a comment, and when the rest of its line closes a bracket that is
/// open before it, the writer meant floor division (a glued `col//3` warns as it is read, warn_on_floor_division_comment).
/// The line, column and the hidden closing bracket.
pub(super) fn comment_hiding_a_closer(input: &str) -> Option<(usize, usize, char)> {
	for (line_index, line) in input.lines().enumerate() {
		let chars: Vec<char> = line.chars().collect();
		let (mut depth, mut quote, mut escaped): (i32, Option<char>, bool) = (0, None, false);
		for (column, &c) in chars.iter().enumerate() {
			match (quote, c) {
				// `\"` inside a text leaves it open
				(Some(_), _) if escaped => escaped = false,
				(Some(_), '\\') => escaped = true,
				(Some(open), _) if c == open => quote = None,
				(Some(_), _) => {}
				(None, '"' | '\'') => quote = Some(c),
				// `# note (x // 2)`: the rest of the line is a comment already
				(None, '#') if chars.get(column + 1).is_none_or(|next| next.is_whitespace()) => break,
				(None, '(' | '[' | '{') => depth += 1,
				(None, ')' | ']' | '}') => depth -= 1,
				(None, '/') if chars.get(column + 1) == Some(&'/') => {
					let spaced = column > 0 && chars[column - 1] == ' ' && chars.get(column + 2) == Some(&' ');
					let rest = &chars[column + 2..];
					let closes = rest
						.iter()
						.map(|c| match c {
							')' | ']' | '}' => 1,
							'(' | '[' | '{' => -1,
							_ => 0,
						})
						.sum::<i32>();
					if spaced && depth > 0 && closes > 0 {
						let closer = rest.iter().find(|c| matches!(c, ')' | ']' | '}')).copied().unwrap_or(')');
						return Some((line_index + 1, column + 1, closer));
					}
					break;
				}
				_ => {}
			}
		}
	}
	None
}

/// `(7 // 2)`: the fix `7 /! 2`, the operands around the `//` at `line`:`column`
pub(super) fn glued_floor_division(diagnostic: Diagnostic, input: &str, line: usize, column: usize) -> Diagnostic {
	let text: Vec<char> = input.lines().nth(line - 1).unwrap_or_default().chars().collect();
	let before = text[..column - 1].iter().rposition(|c| !c.is_whitespace());
	let after = text[column + 1..].iter().position(|c| !c.is_whitespace()).map(|offset| column + 1 + offset);
	let (Some(before), Some(after)) = (before, after) else {
		return diagnostic;
	};
	let written: String = text[before..=after].iter().collect();
	diagnostic.offer("floor division", written, format!("{} {FLOOR_DIVISION} {}", text[before], text[after]))
}

pub(super) fn missing_closer(message: String, closer: impl Into<String>, at: (usize, usize)) -> Node {
	let closer = closer.into();
	Diagnostic::default()
		.message(message)
		.offering(crate::errors::fixits::inserted(format!("the closing {closer}"), closer, at))
		.into_error()
}

impl WarpParser {
	/// `#` followed by a space (`# note`), a shebang `#!`, a doc comment `##` or a directive (`#use lib`, `#include x`, `#import f from "m"`, every import keyword) starts a comment;
	/// `#` directly followed by anything else counts (`#s`, `#a-1`, `#f(x)`)
	pub(super) fn at_hash_comment(&self) -> bool {
		if matches!(self.peek_char(1), ' ' | '\t' | '\n' | '\r' | '\0' | '!' | '#') {
			return true;
		}
		let word: String = (1..).map(|offset| self.peek_char(offset)).take_while(|&c| is_identifier_char(c)).collect();
		crate::modules::is_import_keyword(&word)
	}

	/// `7//2`, `x //= 3`: a `//` glued between operands, or `//=` after one, was meant as floor division; it is a
	/// comment all the same (decision floor-division)
	fn meant_floor_division(&self) -> bool {
		let glued = !self.prev_char().is_whitespace() && !self.peek_char(2).is_whitespace();
		(glued || self.peek_char(2) == '=') && self.follows_operand_on_its_line()
	}

	/// The `//` at the cursor is followed by what a divisor could be: a number or short name (`// 2`, `// n`), not a
	/// word (`// Hz`) or prose
	fn comment_reads_like_divisor(&self) -> bool {
		let rest: String = self.chars[self.pos + 2..].iter().take_while(|&&c| c != '\n').collect();
		is_number_or_short_name(rest.trim())
	}

	pub(super) fn consume_rest_of_line(&mut self) -> String {
		let line_comment = self.consume_to_line_end();
		if self.current_char() == '\n' {
			self.advance();
		}
		line_comment
	}

	/// The rest of the line, trimmed; the newline stays
	fn consume_to_line_end(&mut self) -> String {
		let mut rest = String::new();
		while !matches!(self.current_char(), '\n' | '\0') {
			rest.push(self.current_char());
			self.advance();
		}
		rest.trim().to_string()
	}

	/// A `// …` or `# …` comment ending the line the parser is on: it documents that line's last item, `a: 1 // c`
	/// (card parse-word); the newline stays a separator
	pub(super) fn take_trailing_comment(&mut self) -> Option<String> {
		self.skip_spaces();
		if !self.at_line_comment() {
			return None;
		}
		if self.current_char() == '/' {
			self.warn_on_floor_division_comment();
		}
		self.advance_by(if self.current_char() == '/' { 2 } else { 1 });
		Some(self.consume_to_line_end()).filter(|text| !text.is_empty())
	}

	/// `written` here meant floor division, spelled `replacement` in warp: a warning with that fix, which `warp fix`
	/// applies (`//` reads as a comment, severe; Pascal's `div` still divides, normal; decision floor-division)
	pub(super) fn warn_floor_division_spelling(&self, written: &str, reading: &str, replacement: &str, level: Level) {
		let warning = Diagnostic {
			message: format!("`{written}` {reading}: floor division is written {replacement}"),
			line: self.line_nr,
			column: self.column,
			..Default::default()
		}
		.offer("floor division", written, replacement)
		.about(FLOOR_DIVISION_TOPIC, written)
		.level(level);
		let _ = crate::errors::diagnostic::report(&[warning]);
	}

	/// The `//` at the cursor after code: glued, `7//2` or `x//=3`, it was meant as floor division, a severe warning
	/// whose fix `warp fix` applies; spaced before a divisor, `a // 2`, a got-it note that it is a comment
	fn warn_on_floor_division_comment(&mut self) {
		if self.meant_floor_division() {
			let assigns = self.peek_char(2) == '=';
			let (written, replacement) = if assigns { ("//=", FLOOR_DIVISION_ASSIGN) } else { ("//", FLOOR_DIVISION) };
			self.warn_floor_division_spelling(written, "starts a comment", replacement, Level::Severe);
		} else if self.follows_operand_on_its_line() && !self.is_set_apart() && self.comment_reads_like_divisor() {
			self.set_hint_pos();
			crate::errors::diagnostic::educate_once(
				SLASH_COMMENT_TOPIC,
				"a // b",
				"a /! b",
				"`//` always starts a comment; floor division is /!",
			);
		}
	}

	/// `// note` or `# note` here: a comment to the end of the line, also where floor division was meant (`7//2`
	/// warns); not a URL (`://`) or a
	/// count (`#xs`)
	pub(super) fn at_line_comment(&self) -> bool {
		match (self.current_char(), self.peek_char(1)) {
			('/', '/') => self.prev_char() != ':',
			('#', ' ' | '\t') => true,
			_ => false,
		}
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
				_ if self.at_block_comment_start() => {
					self.consume_block_comment();
				}
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
		let mut after_line_comment = false;
		loop {
			let (newline, indent) = self.skip_whitespace();
			had_newline |= newline;
			if newline {
				line_indent = indent;
			}
			// a blank line ends the comments above: only those directly above an item are its own (a file's header
			// is no doc of its first function, card module-methods); a line comment's newline is consumed with it
			if newline && after_line_comment {
				comments.clear();
			}
			after_line_comment = false;

			let c1 = self.current_char();

			// # line comment (shell-style) at line start: `# note`, `#!`, `##`, `#use …`; `#x`, `#(…)`, `#a-1` count;
			// a // line comment (but not :// URL scheme)
			let marker_width = if c1 == '#' && self.is_at_line_start() && self.at_hash_comment() {
				Some(1)
			} else if c1 == '/' && self.at_line_comment() {
				self.warn_on_floor_division_comment();
				Some(2)
			} else {
				None
			};
			if let Some(width) = marker_width {
				self.advance_by(width);
				let text = self.consume_rest_of_line();
				if !text.is_empty() {
					comments.push(text);
				}
				had_newline = true;
				after_line_comment = true;
				continue;
			}
			// /* block comment */ and /# block comment #/
			if self.at_block_comment_start() {
				let block = self.consume_block_comment();
				had_newline |= block.contains('\n');
				let trimmed = block.trim();
				if !trimmed.is_empty() {
					comments.push(trimmed.to_string());
				}
				continue;
			}
			break;
		}
		let comment = if comments.is_empty() { None } else { Some(comments.join("\n")) };
		(had_newline, line_indent, comment)
	}
}
