//! Units: spaced and glued units `3 m`, time units and durations
use super::*;

impl WarpParser {
	/// `2x` is 2*x, `3(4)` is 3*(4), `3x²` is 3*(x²): a number directly followed by a symbol or `(` multiplies.
	/// Code only: data keeps `size: 3px`. Ordinals `1st` `2nd` `3rd` `4th` are no products.
	pub(crate) fn with_juxtaposed_factor(&mut self, number: Node) -> Node {
		if self.options.data_mode || matches!(number, Node::Error(_)) {
			return number;
		}
		let next = self.current_char();
		// `2ⁿ` raises, it multiplies nothing (try_parse_superscript_power)
		if superscript_letter(next).is_some() {
			return number;
		}
		let tight = (next.is_alphabetic() || next == '_' || next == '(' || next == crate::numbers::units::DEGREE)
			&& !self.at_ordinal_suffix();
		// `1500 meters` stays the pair the unit lowering reads (`3010 meters`), joined so it binds tighter than `>`
		if !tight && self.spaced_word().is_some_and(|word| crate::numbers::units::is_long_unit_name(&word)) {
			self.skip_spaces();
			let unit = self.parse_atom();
			return Node::List(vec![number, unit], Bracket::None, Separator::Space);
		}
		if !tight && !self.at_spaced_unit() {
			return number;
		}
		if !tight {
			self.hint_glued_unit(&number);
		}
		self.skip_spaces();
		let factor = self.parse_atom();
		let factor = self.try_parse_superscript_power(&factor, 0).unwrap_or(factor);
		Node::Key(Box::new(number), Op::Mul, Box::new(factor))
	}

	/// `2 km` multiplies only when `km` is a known unit; any other spaced word keeps the list `[2 foo]`, and so does a
	/// unit word that starts the next entry or assignment: `{w:2 h:3}`; a spaced colon ends a condition instead:
	/// `if x > 1500 m : "far"`. σ joins like a unit: `5 ± 1 σ` is the Gaussian `5 ± 1σ` (card trailing-symbol)
	pub(crate) fn at_spaced_unit(&self) -> bool {
		// `2 m²`: the power is no part of the unit's name
		self.spaced_word().is_some_and(|word| {
			word == crate::numbers::uncertain::SIGMA
				|| crate::numbers::units::names_unit(word.trim_end_matches(|c: char| superscript_digit(c).is_some()))
		})
	}

	/// `[3, 4] cm`: a unit after a list is each element's, the list times the unit (user 2026-10-15, dimensions-units Q3)
	pub(crate) fn with_unit_after_list(&mut self, list: Node) -> Node {
		let is_unit =
			self.spaced_word().is_some_and(|word| word != crate::numbers::uncertain::SIGMA) && self.at_spaced_unit();
		if self.options.data_mode || !is_unit {
			return list;
		}
		self.skip_spaces();
		let unit = self.parse_atom();
		let unit = self.try_parse_superscript_power(&unit, 0).unwrap_or(unit);
		Node::Key(Box::new(list), Op::Mul, Box::new(unit))
	}

	/// `3 km` reads as `3km`, the form units are written in (user 2026-10-09: a soft hint)
	fn hint_glued_unit(&self, number: &Node) {
		let Some(unit) = self.spaced_word().filter(|word| word != crate::numbers::uncertain::SIGMA) else {
			return;
		};
		let amount = number.serialize();
		let (line, column) = self.get_position();
		set_hint_position(line, column);
		crate::normalize::hint(
			&format!("{amount} {unit}"),
			&format!("{amount}{unit}"),
			"a unit is written on its number",
		);
	}

	/// The word after one or more spaces, unless it starts the next entry or assignment
	fn spaced_word(&self) -> Option<String> {
		let rest = &self.chars[self.pos..];
		let spaces = rest.iter().take_while(|c| **c == ' ').count();
		let word: String = rest[spaces..].iter().take_while(|c| is_identifier_char(**c)).collect();
		let after_word = &rest[spaces + word.chars().count()..];
		let next_two: String = after_word.iter().skip_while(|c| **c == ' ').take(2).collect();
		let starts_entry = after_word.first() == Some(&':') || (next_two.starts_with('=') && next_two != "==");
		(spaces > 0 && !word.is_empty() && !starts_entry).then_some(word)
	}

	/// `1 month`, `24 hours`: an integer followed by a time unit word is a duration; `1 month 2 days` is one duration
	pub(crate) fn with_time_unit(&mut self, number: Node) -> Node {
		let Node::Number(Number::Int(count)) = &number else {
			return number;
		};
		if self.options.data_mode {
			return number;
		}
		let Some((unit, length)) = self.time_unit_at(0) else {
			return number;
		};
		self.advance_by(length);
		let mut duration = unit.times(*count);
		while let Some((following, length)) = self.following_duration() {
			self.advance_by(length);
			duration = duration.plus(following);
		}
		Node::data(duration)
	}

	/// The unit word `offset` chars ahead and its length with leading spaces; `{year:1970 month:1}`: a unit word that
	/// starts the next entry or assignment is no unit
	fn time_unit_at(&self, offset: usize) -> Option<(crate::time::Duration, usize)> {
		let (unit, length) = crate::time::unit_after(&self.chars[self.pos + offset..])?;
		let after_word: String =
			self.chars[self.pos + offset + length..].iter().skip_while(|c| **c == ' ').take(2).collect();
		let starts_entry = after_word.starts_with(':') || (after_word.starts_with('=') && after_word != "==");
		(!starts_entry).then_some((unit, length))
	}

	/// ` 2 days` after a duration: the next amount of the same duration, and how many chars it takes
	fn following_duration(&self) -> Option<(crate::time::Duration, usize)> {
		let rest = &self.chars[self.pos..];
		let spaces = rest.iter().take_while(|c| **c == ' ').count();
		let digits = rest[spaces..].iter().take_while(|c| c.is_ascii_digit()).count();
		if spaces == 0 || digits == 0 {
			return None;
		}
		let count: i64 = rest[spaces..spaces + digits].iter().collect::<String>().parse().ok()?;
		let (unit, length) = self.time_unit_at(spaces + digits)?;
		Some((unit.times(count), spaces + digits + length))
	}
}
