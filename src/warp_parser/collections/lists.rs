//! Lists: signed numbers starting a list, arrow bodies in lists
use super::*;

/// `(x => print x)`: `=>` binds looser than the space, so the words after a lambda's arrow are its body (`print x`), not
/// further items; a run of entries `1 => "a" 2 => "b"` stays a list of pairs
pub(crate) fn with_arrow_bodies(mut items: Vec<Node>) -> Vec<Node> {
	let is_arrow = |node: &Node| matches!(node.drop_meta(), Node::Key(_, Op::FatArrow, _));
	let Some(arrow) = items.iter().position(is_arrow) else {
		return items;
	};
	if arrow + 1 == items.len() || items[arrow + 1..].iter().any(is_arrow) {
		return items;
	}
	let rest = items.split_off(arrow + 1);
	let Node::Key(parameters, op, body) = items.pop().expect("the arrow").drop_meta().clone() else {
		unreachable!("an arrow")
	};
	let body = one_expression(&[vec![*body], rest].concat());
	items.push(Node::Key(parameters, op, Box::new(body)));
	items
}

impl WarpParser {
	/// `1 -1`, `[1 +1]`: a number, a space, then a sign glued to a number (D13). Asked: the list `[1 -1]` (the default,
	/// a warning when unanswered) or the arithmetic `1 - 1`. Variables, calls and words keep subtracting: `x -1`.
	/// Inside a list literal a space and a glued sign start the next item after anything (decision list-sign):
	/// `a=5; [a -1]` is [5 -1], while `[a - 1]` and `[a-1]` subtract
	pub(crate) fn signed_number_starts_a_list(&mut self, lhs: &Node) -> Result<bool, Node> {
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
			crate::errors::diagnostic::reading("the list", &format!("{first} ({signed})")), // also inside brackets: `[1 (-1) 2]`
			crate::errors::diagnostic::reading("arithmetic", &format!("{first} {sign} {number}")),
		];
		let question = crate::errors::diagnostic::Ask::new(
			SIGNED_OPERAND_TOPIC,
			format!("is `{written}` a list or arithmetic?"),
			readings,
			crate::errors::diagnostic::Fallback::Warning,
		)
		.written(&written)
		.at(self.line_nr, self.column);
		let is_list = crate::errors::diagnostic::ask(&question)? == 0;
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
}
