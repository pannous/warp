//! Comments: `//`, `#` and `/* */` comments, trailing comments, telling `//` comments from floor division
use super::scanning::is_number_or_short_name;
use super::*;

/// `(col // 3) * 3`: a spaced `//` after code starts a comment, and when the rest of its line closes a bracket that is
/// open before it, the writer meant floor division. The line, column and the hidden closing bracket.
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

/// `(7 // 2)`: the fix `7//2`, the operands around the `//` at `line`:`column` glued to it
pub(super) fn glued_floor_division(diagnostic: Diagnostic, input: &str, line: usize, column: usize) -> Diagnostic {
	let text: Vec<char> = input.lines().nth(line - 1).unwrap_or_default().chars().collect();
	let before = text[..column - 1].iter().rposition(|c| !c.is_whitespace());
	let after = text[column + 1..].iter().position(|c| !c.is_whitespace()).map(|offset| column + 1 + offset);
	let (Some(before), Some(after)) = (before, after) else {
		return diagnostic;
	};
	let written: String = text[before..=after].iter().collect();
	diagnostic.offer("floor division", written, format!("{}//{}", text[before], text[after]))
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

	/// `//` after an operand with no space behind it (`7//2`, `x //= 2`, `f(x) //2`) divides; followed by a space or the end
	/// of the line it starts a comment, as it does after no operand (`//note` on a line of its own, `://` in URLs)
	pub(super) fn at_floor_division(&self) -> bool {
		let divisor_follows = self.chars.get(self.pos + 2).is_some_and(|next| !next.is_whitespace());
		self.current_char() == '/' && self.peek_char(1) == '/' && divisor_follows && self.follows_operand_on_its_line()
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
		self.educate_on_spaced_floor_division();
		self.advance_by(if self.current_char() == '/' { 2 } else { 1 });
		Some(self.consume_to_line_end()).filter(|text| !text.is_empty())
	}

	/// `a // b` after code reads like division: say once that it is a comment
	fn educate_on_spaced_floor_division(&mut self) {
		if self.current_char() == '/'
			&& self.follows_operand_on_its_line()
			&& !self.is_set_apart()
			&& self.comment_reads_like_divisor()
		{
			self.set_hint_pos();
			crate::errors::diagnostic::educate_once(
				SLASH_COMMENT_TOPIC,
				"a // b",
				"a//b",
				"`// …` after code is a comment; floor division has no space after //: a //b",
			);
		}
	}

	/// `// note` or `# note` here: a comment to the end of the line, not floor division (`7//2`), a URL (`://`) or a
	/// count (`#xs`)
	pub(super) fn at_line_comment(&self) -> bool {
		match (self.current_char(), self.peek_char(1)) {
			('/', '/') => self.prev_char() != ':' && !self.at_floor_division(),
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

			// # line comment (shell-style) at line start: `# note`, `#!`, `##`, `#use …`; `#x`, `#(…)`, `#a-1` count
			if c1 == '#' && self.is_at_line_start() && self.at_hash_comment() {
				self.advance();
				let text = self.consume_rest_of_line();
				if !text.is_empty() {
					comments.push(text);
				}
				had_newline = true;
				after_line_comment = true;
				continue;
			}
			// // line comment (but not :// URL scheme)
			if c1 == '/' && self.at_line_comment() {
				self.educate_on_spaced_floor_division();
				self.advance_by(2);
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
