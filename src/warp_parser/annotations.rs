//! Annotations: `@attribute` and trailing attributes, `@x` instance variables
use super::*;
use crate::words::SELF_WORD;

impl WarpParser {
	/// `@name` or `@name(value)` annotates the atom that follows: `@version(2) @draft tee{a:1}`.
	/// `@name:value` (inside a literal: `point{x:1 @source:"gps"}`) is the meta entry `@name`, never a field;
	/// after a dot, `p.@name` names the meta key itself.
	pub(super) fn parse_attribute(&mut self) -> Node {
		let after_dot = self.pos > 0 && self.chars[self.pos - 1] == '.';
		self.advance(); // skip '@'
		let name = or_return_error!(self.parse_symbol());
		if after_dot {
			return Symbol(format!("{ATTRIBUTE_MARK}{name}"));
		}
		if self.type_fields.is_some() && self.reads_instance_variable() {
			return Node::Key(Box::new(Symbol(SELF_WORD.to_string())), Op::Dot, Box::new(Symbol(name)));
		}
		if self.current_char() == ':' && self.peek_char(1) != '=' {
			self.advance();
			self.skip_spaces();
			return Node::Key(
				Box::new(Symbol(format!("{ATTRIBUTE_MARK}{name}"))),
				Op::Colon,
				Box::new(self.parse_atom()),
			);
		}
		let value = self.attribute_value();
		self.parse_atom().with_attribute(&name, value)
	}

	/// Ruby's instance variable `@x` in a class body, after its name: an operator, the end of the statement, or the line
	/// end before an `end` follows (an annotation `@deprecated fun f()` stands before a word or its own value)
	fn reads_instance_variable(&self) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		match self.peek_char(blanks) {
			'\n' => {
				let next_line =
					blanks
						+ 1 + (0..).take_while(|&offset| matches!(self.peek_char(blanks + 1 + offset), ' ' | '\t')).count();
				let word: String = (next_line..)
					.map(|offset| self.peek_char(offset))
					.take_while(|ch| is_identifier_char(*ch))
					.collect();
				word == END_KEYWORD
			}
			'(' => false,
			next => !is_identifier_char(next) && next != '@',
		}
	}

	/// The value of `@name(value)`, true for a bare `@name`
	fn attribute_value(&mut self) -> Node {
		if self.current_char() == '(' {
			Self::unwrap_single(self.parse_bracketed('('))
		} else {
			Node::True
		}
	}

	/// `ys = xs.map(f) @parallel`: an `@name` or `@name(value)` with nothing after it on the statement annotates the
	/// expression before it (wiki/Purpose.md), where a leading one would annotate an empty atom
	pub(super) fn try_parse_trailing_attribute(&mut self, annotated: &Node) -> Option<Node> {
		if self.current_char() != '@' || !self.peek_char(1).is_alphabetic() {
			return None;
		}
		let name = self.word_at(1);
		let mut end = 1 + name.chars().count();
		if self.peek_char(end) == '(' {
			let mut depth = 0;
			loop {
				match self.peek_char(end) {
					'(' => depth += 1,
					')' if depth == 1 => break,
					')' => depth -= 1,
					'\0' | '\n' => return None,
					_ => {}
				}
				end += 1;
			}
			end += 1;
		}
		let after = end + (end..).take_while(|&at| matches!(self.peek_char(at), ' ' | '\t')).count();
		if !matches!(self.peek_char(after), '\0' | '\n' | '\r' | ';' | ',' | ')' | ']' | '}') {
			return None;
		}
		self.advance_by(1 + name.chars().count());
		let value = self.attribute_value();
		Some(annotated.clone().with_attribute(&name, value))
	}
}
