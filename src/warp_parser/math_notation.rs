//! Math notation: superscript powers `x²`, `xⁿ`, norm bars `‖v‖`
use super::*;

/// The sign of a superscript exponent: ⁺ is +1, ⁻ is -1
pub(crate) fn superscript_sign(ch: char) -> Option<i64> {
	match ch {
		'⁺' => Some(1),
		'⁻' => Some(-1),
		_ => None,
	}
}

pub(crate) fn superscript_letter(ch: char) -> Option<char> {
	SUPERSCRIPT_LETTERS.iter().find(|(raised, _)| *raised == ch).map(|(_, letter)| *letter)
}

pub(crate) fn superscript_digit(ch: char) -> Option<i64> {
	SUPERSCRIPT_DIGITS.chars().position(|digit| digit == ch).map(|position| position as i64)
}

impl WarpParser {
	/// `‖x‖` brackets a whole expression like parentheses: `‖3-5‖*2` → 4
	pub(super) fn parse_norm_bars(&mut self) -> Node {
		let inner = self.parse_expr(0);
		self.skip_spaces();
		if self.current_char() != '‖' {
			return missing_closer(
				"Missing closing ‖".into(),
				"‖",
				crate::errors::fixits::end_of_line(&self.input, self.line_nr),
			);
		}
		self.advance();
		self.finish_prefix(Op::Abs, inner)
	}

	/// The exponent written in superscript digits and signs at the cursor, its length in characters and whether it has a sign:
	/// ⁴ → (4, 1), ¹² → (12, 2), ⁻¹ → (-1, 2), ²⁺³ → (5, 3)
	pub(super) fn superscript_exponent(&self) -> Option<(i64, usize, bool)> {
		let (mut exponent, mut length, mut signed) = (0i64, 0usize, false);
		loop {
			let sign = match self.chars.get(self.pos + length).copied().and_then(superscript_sign) {
				Some(sign) => sign,
				None if length == 0 => 1,
				None => break,
			};
			let sign_length = self.chars.get(self.pos + length).copied().and_then(superscript_sign).map_or(0, |_| 1);
			let digits: Vec<i64> = self
				.chars
				.iter()
				.skip(self.pos + length + sign_length)
				.map_while(|ch| superscript_digit(*ch))
				.collect();
			if digits.is_empty() {
				break;
			}
			let run = digits.iter().fold(0i64, |run, digit| run.saturating_mul(10).saturating_add(*digit));
			exponent = exponent.saturating_add(sign * run);
			signed |= sign_length > 0;
			length += sign_length + digits.len();
		}
		(length > 0).then_some((exponent, length, signed))
	}

	/// The exponent of superscript terms with a letter among them: `ⁿ` → n, `ⁿ⁺ᵐ` → n+m, `ⁿ⁺¹` → n+1; its length in
	/// characters and whether it starts with ⁻
	pub(super) fn superscript_variable_exponent(&self) -> Option<(Node, usize, bool)> {
		let at = |offset: usize| self.chars.get(self.pos + offset).copied();
		let (mut terms, mut length, mut has_letter) = (vec![], 0usize, false);
		loop {
			let sign = at(length).and_then(superscript_sign);
			let start = length + usize::from(sign.is_some());
			let letters: String = (start..).map_while(|offset| at(offset).and_then(superscript_letter)).collect();
			let digits: Vec<i64> = (start..).map_while(|offset| at(offset).and_then(superscript_digit)).collect();
			let term = if !letters.is_empty() {
				has_letter = true;
				length = start + letters.chars().count();
				Node::Symbol(letters)
			} else if !digits.is_empty() {
				length = start + digits.len();
				Node::int(digits.iter().fold(0i64, |run, digit| run.saturating_mul(10).saturating_add(*digit)))
			} else {
				break;
			};
			terms.push((sign.unwrap_or(1), term));
		}
		if !has_letter {
			return None;
		}
		let negative = terms.first().is_some_and(|(sign, _)| *sign < 0);
		let mut terms = terms.into_iter();
		let (_, first) = terms.next()?;
		let exponent = terms.fold(first, |sum, (sign, term)| {
			Node::Key(Box::new(sum), if sign < 0 { Op::Sub } else { Op::Add }, Box::new(term))
		});
		Some((exponent, length, negative))
	}

	/// `x⁴` is x^4, `x⁻¹` is 1/x, `2ⁿ` is 2^n; a lone ² or ³ is x^2 or x^3 binding as tight as a suffix (`-x²` is -(x^2))
	pub(super) fn try_parse_superscript_power(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		if let Some((exponent, length, negative)) = self.superscript_variable_exponent() {
			if Op::Pow.binding_power().0 < min_bp {
				return None;
			}
			self.advance_by(length);
			let power = Node::Key(Box::new(lhs.clone()), Op::Pow, Box::new(exponent));
			return Some(if negative { Node::Key(Box::new(Node::int(1)), Op::Div, Box::new(power)) } else { power });
		}
		let (exponent, length, signed) = self.superscript_exponent()?;
		let binding_op = if matches!((exponent, length, signed), (2 | 3, 1, false)) { Op::Inc } else { Op::Pow };
		if binding_op.binding_power().0 < min_bp {
			return None;
		}
		self.advance_by(length);
		let power = Node::Key(Box::new(lhs.clone()), Op::Pow, Box::new(Node::int(exponent.abs())));
		Some(if exponent < 0 { Node::Key(Box::new(Node::int(1)), Op::Div, Box::new(power)) } else { power })
	}
}
