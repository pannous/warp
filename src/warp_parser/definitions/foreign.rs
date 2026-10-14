//! Foreign declaration modifiers: `public static fun f()`, `private x = 4`

use super::*;

impl WarpParser {
	/// `public static fun f(){…}`, `private x = 4` (P78): a run of modifiers that ends before a function definition, or of
	/// modifiers without warp meaning before a name, is skipped, each meaningless one with a note; the statement after it
	pub(crate) fn skip_foreign_modifiers(&mut self, first: &str) -> Option<Node> {
		let is_modifier = |word: &str| FOREIGN_MODIFIERS.contains(&word) || DEFINITION_MODIFIERS.contains(&word);
		if !is_modifier(first) {
			return None;
		}
		let mut words = vec![first.to_string()];
		let mut offset = 0;
		let next = loop {
			let blanks = (offset..)
				.take_while(|at| matches!(self.peek_char(*at), ' ' | '\t'))
				.count();
			if blanks == 0 {
				return None; // `public = 3`, `public(…)`: a name
			}
			let start = offset + blanks;
			let word: String = (start..)
				.map(|at| self.peek_char(at))
				.take_while(|ch| is_identifier_char(*ch))
				.collect();
			offset = start + word.chars().count();
			if !is_modifier(&word) {
				break word;
			}
			words.push(word);
		};
		let before_definition = crate::operators::is_function_keyword(&next);
		let foreign_only = words.iter().all(|word| FOREIGN_MODIFIERS.contains(&word.as_str()));
		if next.is_empty() || !(before_definition || foreign_only) {
			return None;
		}
		let (line, column) = (self.line_nr, self.column.saturating_sub(first.chars().count()));
		let is_static = words.iter().any(|word| word == STATIC_KEYWORD);
		for word in words
			.iter()
			.filter(|word| FOREIGN_MODIFIERS.contains(&word.as_str()) && *word != STATIC_KEYWORD)
		{
			let question = Ask::new(
				FOREIGN_MODIFIER_TOPIC,
				format!("{word} has no meaning in warp"),
				vec![reading("skip it", &next)],
				Fallback::Warning,
			)
			.written(word)
			.at(line, column);
			if let Err(error) = ask(&question) {
				return Some(error);
			}
		}
		self.advance_by(offset - next.chars().count());
		self.skip_spaces();
		let statement = self.parse_expr(0);
		Some(if is_static {
			statement.with_attribute(STATIC_KEYWORD, Node::True)
		} else {
			statement
		})
	}
}
