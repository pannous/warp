//! Match arms: `else` and `in` keys

use super::*;

impl WarpParser {
	/// `else => …` at the cursor: the catch-all arm of a match on its own line, no else branch of the line before
	pub(crate) fn else_arm_key(&self) -> bool {
		let after: String = self.chars[(self.pos + 4).min(self.chars.len())..]
			.iter()
			.skip_while(|ch| matches!(ch, ' ' | '\t'))
			.take(2)
			.collect();
		after == "=>"
	}

	/// The cursor is in the key of a match arm: `=>` follows before the statement or block ends (`1 | 2 | 3 => …`)
	pub(crate) fn in_arm_key(&self) -> bool {
		let rest = &self.chars[self.pos.min(self.chars.len())..];
		let end = rest.iter().position(|ch| matches!(ch, ';' | '\n' | '{' | '}')).unwrap_or(rest.len());
		rest[..end].windows(2).any(|pair| pair == ['=', '>'])
	}
}
