//! User-declared operators: prefix, infix and suffix operators and their precedence declarations

use super::*;

impl WarpParser {
	/// The declared operator written at the cursor, of one of the kinds
	pub(super) fn user_operator_at(&self, kinds: &[UserOperatorKind]) -> Option<UserOperator> {
		self.user_operators.iter().find(|operator| kinds.contains(&operator.kind) && self.chars[self.pos..].starts_with(&operator.glyph)).cloned()
	}

	/// `3‼`: the call of the suffix operator on the left operand
	pub(super) fn try_parse_user_suffix(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		let operator = self.user_operator_at(&[UserOperatorKind::Suffix])?;
		if min_bp > operator.level {
			return None;
		}
		self.advance_by(operator.glyph.len());
		Some(operator_call(&operator.glyph, vec![lhs.clone()]))
	}

	/// `2 ⊕ 3`: the call of the infix operator on both operands
	pub(super) fn try_parse_user_infix(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		let operator = self.user_operator_at(&[UserOperatorKind::Infix])?;
		let (left_bp, right_bp) = (operator.level, operator.level + 1);
		if left_bp < min_bp {
			return None;
		}
		self.advance_by(operator.glyph.len());
		self.skip_whitespace();
		let rhs = self.parse_expr(right_bp);
		Some(operator_call(&operator.glyph, vec![lhs.clone(), rhs]))
	}

	/// `∆3`: the call of the prefix operator on the operand after it
	pub(super) fn try_parse_user_prefix(&mut self) -> Option<Node> {
		let operator = self.user_operator_at(&[UserOperatorKind::Prefix])?;
		self.advance_by(operator.glyph.len());
		self.skip_spaces();
		let operand = self.parse_expr(operator.level);
		Some(operator_call(&operator.glyph, vec![operand]))
	}

	/// `prefix|suffix|infix operator ⊕ := body` defines the function `operator_⊕`: its parameter is `it` (prefix, suffix) or `a b` (infix).
	/// `operator ⊕ has precedence above +` is not supported.
	pub(super) fn try_parse_operator_declaration(&mut self, word: &str) -> Option<Node> {
		if self.options != ParserOptions::default() {
			return None;
		}
		let rest: String = self.chars[self.pos..].iter().take_while(|c| **c != '\n' && **c != ';').collect();
		let words: Vec<&str> = rest.split_whitespace().collect();
		if word == "operator" && words.get(1) == Some(&"has") && words.get(2) == Some(&"precedence") {
			while !matches!(self.current_char(), '\n' | ';' | '\0') {
				self.advance();
			}
			// the pre-scan applied it (scan_user_operators); here only its error is reported
			let statement: Vec<&str> = std::iter::once("operator").chain(words.iter().copied()).collect();
			return Some(match precedence_declaration(&statement, &self.user_operators) {
				Ok(Some(_)) => Empty,
				Ok(None) => error(&format!("an operator precedence is declared `operator ⊕ has precedence above|below Y`, not `operator {}`", words.join(" "))),
				Err(message) => error(&message),
			});
		}
		let (_, kind) = OPERATOR_KINDS.iter().find(|(keyword, _)| *keyword == word)?;
		let glyph = declared_glyph(&words).filter(|glyph| is_operator_glyph(glyph))?;
		let has_operator_word = words.first() == Some(&"operator");
		let glyph: Vec<char> = glyph.chars().collect();
		self.skip_spaces();
		if has_operator_word {
			self.advance_by("operator".chars().count());
			self.skip_spaces();
		}
		self.advance_by(glyph.len());
		self.skip_spaces();
		self.advance_by(":=".len());
		let body = self.parse_expr(0);
		let parameters = match kind {
			// P48 names the operands `left` and `right`; `a` and `b` stay accepted
			UserOperatorKind::Infix => {
				let names = if mentions(&body, "left") || mentions(&body, "right") { ["left", "right"] } else { ["a", "b"] };
				names.map(|name| Symbol(name.to_string())).to_vec()
			}
			_ => vec![Symbol("it".to_string())],
		};
		let head = operator_call(&glyph, parameters);
		Some(Node::Key(Box::new(head), Op::Define, Box::new(body)))
	}

	/// Hint when the operator written at the cursor is not the canonical one; call once when the operator is consumed
	/// The operator as written, after hinting at its preferred spelling
	pub(super) fn hint_operator(&self, chars: usize, is_prefix: bool) -> String {
		let written: String = (0..chars).map(|offset| self.peek_char(offset)).collect();
		self.set_hint_pos();
		norm::operator(&written, is_prefix);
		written
	}

	/// Ranges whose end readers expect either way are Asks, by default read as warp does (exclusive):
	/// `a upto b` (wiki/range.md: excludes b) and the loop bound `for i in 0..n-1` (Kotlin's `..` includes n-1)
	pub(super) fn range_reading(&self, op: Op, written: &str, end: &Node, line: usize, column: usize) -> Result<Op, Node> {
		let end_text = crate::normalize::operand_text(end);
		let question = match written {
			UPTO => format!("does `upto {end_text}` include {end_text}? (`..<` or `..` exclude it, `to` or `...` include it)"),
			EXCLUSIVE_DOTS if self.in_for_header && is_minus_one(end) => {
				format!("does the loop bound `..{end_text}` include {end_text}? (warp's `..` excludes it, Kotlin's includes it)")
			}
			_ => return Ok(op),
		};
		let topic = if written == UPTO { UPTO } else { KOTLIN_RANGE };
		let readings = vec![reading("exclusive", "..<"), reading("inclusive", "...")];
		let chosen = ask(&Ask::new(topic, question, readings, Fallback::Warning).written(written).at(line, column))?;
		Ok(if chosen == 0 { Op::Range } else { Op::To })
	}
}
