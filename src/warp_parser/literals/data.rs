//! Data literals: structured data read as a value
use super::*;

impl WarpParser {
	/// `data a and b` is the data `a and b`, also as an operand (`x = data a and b`, `string(data a and b)`); parentheses
	/// around all of it only group it (`data (a and b)`). `data = …`, `data.x`, glued `data(x)` and `data class` stay
	pub(crate) fn try_parse_data(&mut self) -> Option<Node> {
		// data is a keyword (card data-reserved): `data = …` is refused by soft_keywords.rs, never read as a variable
		if self.options.data_mode || !self.matches_keyword(DATA_KEYWORD) || self.peek_char(DATA_KEYWORD.len()) != ' ' {
			return None;
		}
		let before_keyword = self.mark();
		self.advance_by(DATA_KEYWORD.len());
		self.skip_spaces();
		let ends = matches!(self.current_char(), '}' | ']' | ')' | ';' | ',' | '\n' | '\r' | '\0');
		if ends || self.peek_operator().is_some() || self.matches_keyword(CLASS_KEYWORD) {
			self.rewind(before_keyword);
			return None;
		}
		let parsed = self.parse_expr(Op::Assign.binding_power().1);
		let data = match parsed.drop_meta() {
			Node::List(items, Bracket::Round, _) if items.len() == 1 => items[0].clone(),
			_ => parsed,
		};
		Some(Node::List(vec![Symbol(DATA_KEYWORD.to_string()), data], Bracket::None, Separator::Space))
	}
}
