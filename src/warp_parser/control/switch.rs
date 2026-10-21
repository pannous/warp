//! Match arms: `else` and `in` keys, a match as an arm's value

use super::*;
use crate::words::SWITCH_WORDS;

/// What follows the key of a match arm
const ARM_ARROW: &str = "=>";

impl WarpParser {
	/// `else => …` at the cursor: the catch-all arm of a match on its own line, no else branch of the line before
	pub(crate) fn else_arm_key(&self) -> bool {
		self.text_at(ELSE_KEYWORD.len() + self.blanks_at(ELSE_KEYWORD.len()), ARM_ARROW)
	}

	/// The cursor is in the key of a match arm: `=>` follows before the statement or block ends (`1 | 2 | 3 => …`)
	pub(crate) fn in_arm_key(&self) -> bool {
		let rest = &self.chars[self.pos.min(self.chars.len())..];
		let end = rest.iter().position(|ch| matches!(ch, ';' | '\n' | '{' | '}')).unwrap_or(rest.len());
		rest[..end].windows(2).any(|pair| pair == ['=', '>'])
	}

	/// `pattern: match v {…}`: the arm's value is the whole inner match, its subject and its cases on the line, though
	/// `:` binds tighter than the words after it (card match-arm-whose; twin: parser.warp with_switch_cases)
	pub(crate) fn with_switch_cases(&mut self, value: Node, subject_bp: u8) -> Node {
		let opens_switch = matches!(value.drop_meta(), Node::Symbol(word) if SWITCH_WORDS.contains(&word.as_str()));
		if !opens_switch || self.options.data_mode || !self.braceless_argument_follows() {
			return value;
		}
		let mut parts = vec![value, self.parse_expr(subject_bp)];
		self.skip_spaces();
		if self.current_char() == '{' {
			parts.push(self.parse_bracketed('{'));
		}
		Node::List(parts, Bracket::None, Separator::Space)
	}
}
