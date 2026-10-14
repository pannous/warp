//! Suffixes: superscripts, suffix operators, bang and test words, subscripts, slices, implicit application

use super::*;

/// The marks of a slice from the start, `s[…3]`: the inclusive ones before `..`, which they begin with
const SLICE_FROM_START: [(&str, Op); 4] = [(":", Op::Colon), ("…", Op::To), ("...", Op::To), ("..", Op::Range)];
/// The operands a braceless call binds in: from the right side of `or` to the right side of `+`; a `*` operand keeps
/// `h*w int` the typed array `(h*w) int`
const OPERAND_APPLICATION_BPS: std::ops::RangeInclusive<u8> = Op::Or.binding_power().1..=Op::Add.binding_power().1;

impl WarpParser {
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
			let sign_length = self
				.chars
				.get(self.pos + length)
				.copied()
				.and_then(superscript_sign)
				.map_or(0, |_| 1);
			let digits: Vec<i64> = self
				.chars
				.iter()
				.skip(self.pos + length + sign_length)
				.map_while(|ch| superscript_digit(*ch))
				.collect();
			if digits.is_empty() {
				break;
			}
			let run = digits
				.iter()
				.fold(0i64, |run, digit| run.saturating_mul(10).saturating_add(*digit));
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
			let letters: String = (start..)
				.map_while(|offset| at(offset).and_then(superscript_letter))
				.collect();
			let digits: Vec<i64> = (start..)
				.map_while(|offset| at(offset).and_then(superscript_digit))
				.collect();
			let term = if !letters.is_empty() {
				has_letter = true;
				length = start + letters.chars().count();
				Node::Symbol(letters)
			} else if !digits.is_empty() {
				length = start + digits.len();
				Node::int(
					digits
						.iter()
						.fold(0i64, |run, digit| run.saturating_mul(10).saturating_add(*digit)),
				)
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

	/// `x⁴` is x^4, `x⁻¹` is 1/x, `2ⁿ` is 2^n; the single digits ² and ³ keep their dedicated square and cube operators
	pub(super) fn try_parse_superscript_power(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		if let Some((exponent, length, negative)) = self.superscript_variable_exponent() {
			if Op::Pow.binding_power().0 < min_bp {
				return None;
			}
			self.advance_by(length);
			let power = Node::Key(Box::new(lhs.clone()), Op::Pow, Box::new(exponent));
			return Some(if negative {
				Node::Key(Box::new(Node::int(1)), Op::Div, Box::new(power))
			} else {
				power
			});
		}
		let (exponent, length, signed) = self.superscript_exponent()?;
		let (op, right) = match (exponent, length, signed) {
			(2, 1, false) => (Op::Square, Empty),
			(3, 1, false) => (Op::Cube, Empty),
			_ => (Op::Pow, Node::int(exponent.abs())),
		};
		if op.binding_power().0 < min_bp {
			return None;
		}
		self.advance_by(length);
		let power = Node::Key(Box::new(lhs.clone()), op, Box::new(right));
		Some(if exponent < 0 {
			Node::Key(Box::new(Node::int(1)), Op::Div, Box::new(power))
		} else {
			power
		})
	}

	pub(super) fn try_parse_suffix(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		if let Some(power) = self.try_parse_superscript_power(lhs, min_bp) {
			return Some(power);
		}
		let (op, chars) = self.peek_suffix_operator()?;
		let (l_bp, _) = op.binding_power();
		if l_bp < min_bp {
			return None;
		}
		self.advance_by(chars);
		Some(Node::Key(Box::new(lhs.clone()), op, Box::new(Empty)))
	}

	/// `{a*a}!` and `f!` at the end of a statement evaluate the block or name, which a block does on the spot anyway.
	/// After a function or method the `!` mutates in place (user decision D2, by position): `x.upper!` and `upper(x)!`
	/// assign the result back to x; in `upper x!` the name is marked for crate::mutation to do the same.
	pub(super) fn try_parse_evaluate_bang(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		// `xs#2!` runs xs#2, not the index 2: the suffix binds looser than `#`, tighter than arithmetic (`x!+1`)
		if min_bp > BANG_BP {
			return None;
		}
		let mutated = crate::mutation::mutated_variable(lhs).cloned();
		// `o.s1!`: a field (no method) may hold a block too (wiki/charged.md section 4)
		// any expression may be run (wiki/charged.md section 5): `xs#2!`, `f(x)!`; Empty too (`ø!` P73 force → unwrap)
		// `name! email?`: glued to its name and followed by a space, the `!` is a suffix even when an operand follows
		// `x!+1`: glued to its name and followed by an infix operator, the `!` is a suffix too
		// `x!!`: run fully, a suffix as well
		let glued_suffix = !self.prev_char().is_whitespace()
			&& (matches!(self.peek_char(1), ' ' | '\t' | '!') || INFIX_AFTER_BANG.contains(&self.peek_char(1)));
		if self.current_char() != '!' || self.peek_char(1) == '=' || (self.operand_follows(1) && !glued_suffix) {
			return None;
		}
		self.advance();
		// `x!!` runs a block fully, nested blocks too (wiki/charged.md section 5)
		let fully = self.current_char() == '!' && self.peek_char(1) != '=';
		if fully {
			self.advance();
		}
		Some(match (mutated, lhs.drop_meta()) {
			// `o.s1!` runs a block field, `x.upper!` mutates (D2): blocks.rs knows which, mutation.rs mutates the rest
			(Some(_), Node::Key(_, Op::Dot, _)) if fully => crate::mutation::marked_fully(lhs.clone()),
			(Some(_), Node::Key(_, Op::Dot, _)) => crate::mutation::marked(lhs.clone()),
			(Some(variable), _) => Node::Key(Box::new(variable), Op::Assign, Box::new(lhs.clone())),
			(None, Node::Symbol(_)) if fully => crate::mutation::marked_fully(lhs.clone()),
			(None, Node::Symbol(_)) => crate::mutation::marked(lhs.clone()),
			// `{a*a}!` evaluates the block on the spot
			(None, Node::List(_, Bracket::Curly, _)) => lhs.clone(),
			// `ø!`: P73 force → unwrap (direct call; marking Empty is stripped by is_nothing / run_time_blocks)
			(None, Node::Empty) => call(crate::words::UNWRAP, vec![lhs.clone()]),
			(None, _) if fully => crate::mutation::marked_fully(lhs.clone()),
			(None, _) => crate::mutation::marked(lhs.clone()),
		})
	}

	pub(super) fn try_parse_subscript(&mut self, lhs: &Node, min_bp: u8, subscript_bp: u8) -> Option<Node> {
		// a space before the bracket makes it a list, `f [1, 2]`; only `xs[1]` indexes
		if self.current_char() != '['
			|| min_bp > subscript_bp
			|| self.prev_char().is_whitespace()
			|| is_unindexable_keyword(lhs)
		{
			return None;
		}
		if let Some(list_type) = self.try_parse_array_type_suffix(lhs) {
			return Some(list_type);
		}
		let before_bracket = self.mark();
		self.advance(); // skip '['
		self.skip_whitespace();

		let outer_spaced_list = std::mem::replace(&mut self.in_spaced_list, false); // `xs[i -1]` subtracts
		let mut indices = vec![self.parse_slice_from_start().unwrap_or_else(|| self.parse_expr(0))];
		self.skip_whitespace();

		while self.current_char() == ',' {
			self.advance(); // skip ','
			self.skip_whitespace();
			indices.push(self.parse_expr(0));
			self.skip_whitespace();
		}
		self.in_spaced_list = outer_spaced_list;

		if self.current_char() != ']' {
			self.rewind(before_bracket); // `foo [1 2 3]`: a list argument, not an index
			return None;
		}
		self.advance(); // skip ']'
		if let Some(node) = self.try_attributed_body(lhs, &indices) {
			return Some(node);
		}
		// `int[n]` is n zeros of the type unless int is a variable (analyzer lower_declarations): no indexing hint
		let names_a_type =
			matches!(lhs.drop_meta(), Node::Symbol(word) if crate::declaration_forms::zero_list(Empty, word).is_some());
		if slice_bounds(&indices[0]).is_none() && !names_a_type && !self.names_a_key(&indices[0]) {
			crate::normalize::set_position_of(lhs);
			norm::index_operator(
				&crate::normalize::operand_text(lhs),
				&crate::normalize::operand_text(&indices[0]),
				true,
			);
		}

		Some(indices.into_iter().fold(lhs.clone(), subscript))
	}

	/// `a[id=1]{…}` (wiki/reference.md): attributes in brackets before a body are its meta entries, `a{@id:1 …}`
	fn try_attributed_body(&mut self, lhs: &Node, indices: &[Node]) -> Option<Node> {
		let attributes: Vec<Node> = indices
			.iter()
			.flat_map(attribute_items)
			.map(|item| match item.drop_meta() {
				Node::Key(key, Op::Assign | Op::Colon, value) if matches!(key.drop_meta(), Symbol(_)) => {
					Some(Node::Key(
						Box::new(Symbol(format!("{ATTRIBUTE_MARK}{}", key.name()))),
						Op::Colon,
						value.clone(),
					))
				}
				_ => None,
			})
			.collect::<Option<_>>()?;
		let mut offset = 0;
		while self.peek_char(offset) == ' ' {
			offset += 1;
		}
		if !matches!(lhs.drop_meta(), Symbol(_)) || self.peek_char(offset) != '{' {
			return None;
		}
		self.skip_spaces();
		let Node::List(items, bracket, separator) = self.parse_bracketed('{') else {
			return None;
		};
		let body = Node::List([attributes, items].concat(), bracket, separator);
		Some(Node::Key(Box::new(lhs.clone()), Op::Colon, Box::new(body)))
	}

	/// `[:end]`, `[…last]`, `[..end]` and `[:]`, a slice from the start: `ø:end`
	pub(super) fn parse_slice_from_start(&mut self) -> Option<Node> {
		let (op, chars) = SLICE_FROM_START
			.iter()
			.copied()
			.find(|(written, _)| {
				written
					.chars()
					.enumerate()
					.all(|(offset, ch)| self.peek_char(offset) == ch)
			})
			.map(|(written, op)| (op, written.chars().count()))?;
		self.advance_by(chars);
		self.skip_whitespace();
		let end = if self.current_char() == ']' {
			Empty
		} else {
			self.parse_expr(0)
		};
		Some(Node::Key(Box::new(Empty), op, Box::new(end)))
	}

	/// The Java/C array type `int[]` is the list type `[int]`
	pub(super) fn try_parse_array_type_suffix(&mut self, lhs: &Node) -> Option<Node> {
		let Node::Symbol(word) = lhs.drop_meta() else {
			return None;
		};
		if !is_identifier_char(self.prev_char()) || self.peek_char(1) != ']' || !names_a_type(word) {
			return None;
		}
		self.advance_by(2);
		Some(Node::List(vec![lhs.clone()], Bracket::Square, Separator::Space))
	}

	/// The word at the cursor begins what follows the operand rather than being its argument: a loop clause
	/// (`[x - y for x in xs]`), a block's `end` (`return a + b end`), a key (`inner  _: 0`) or an element (`n p{m}`)
	fn word_begins_item(&self) -> bool {
		if self.matches_keyword(crate::words::FOR_WORD) || self.matches_keyword(END_KEYWORD) {
			return true;
		}
		let length = (0..).take_while(|at| is_identifier_char(self.peek_char(*at))).count();
		match (self.peek_char(length), self.peek_char(length + 1)) {
			('{', _) => length > 0,
			(':', next) => length > 0 && !matches!(next, ':' | '='),
			_ => false,
		}
	}

	pub(super) fn try_parse_implicit_application(
		&mut self,
		lhs: &Node,
		min_bp: u8,
		argument_bp: u8,
		max_bp_for_application: u8,
	) -> Option<Node> {
		let lhs_is_callable = match lhs.drop_meta() {
			Node::Symbol(_) => true,
			Node::List(_, Bracket::None, _) => true, // Function call from implicit application
			_ => false,
		};
		let lhs_is_defined_function = matches!(lhs.drop_meta(), Node::Symbol(name) if self.functions.contains(name));
		// application binds tighter than the logical, comparison and additive operators: `1 + count xs` is
		// `1 + count(xs)` for any name (card count-xs); a `:` operand keeps `a: b c d` apart, and an argument takes no
		// call of its own: `on change s` is one call with two arguments; a loop header's last word is no call of its body
		// (`for i in 1 to n body`), and a word that begins the next item is no argument (word_begins_item)
		let in_operand = OPERAND_APPLICATION_BPS.contains(&min_bp)
			&& min_bp != argument_bp
			&& !self.in_for_header
			&& !self.word_begins_item();
		let ch = self.current_char();
		let in_assignment_context = min_bp <= 60 || self.branch_bp == Some(min_bp); // Assignment r_bp is 59
		let arg_is_non_identifier =
			ch.is_numeric()
				|| self.number_starts_at(0)
				|| ch == '"' || ch == '\''
				|| ch == '(' || ch == '['
				|| ch == '{' || ch == '-'
				|| self.starts_function_reference();
		let applies_otherwise = in_assignment_context || arg_is_non_identifier || lhs_is_defined_function;
		let should_apply = in_operand || applies_otherwise;

		// At statement level a list `f a b` is a call with all its items; a function of the implicit `it` takes one argument,
		// so `f 3-1 > 15` compares `f(3-1)` just like the operand `1 + f 3-1 > 15` does
		let takes_one_argument = lhs_is_defined_function && !self.functions_with_parameters.contains(&lhs.name());
		if (min_bp == 0 && !takes_one_argument) || min_bp > max_bp_for_application || self.glued_pair_bp == Some(min_bp)
		{
			return None;
		}
		if !lhs_is_callable || !self.can_start_atom() || !should_apply || (ch == '{' && self.in_for_header) {
			return None;
		}

		let start = self.pos;
		// `while i < count t {…}`: a block after an operand's argument is the body, as after a loop header's
		let outer_header = self.in_for_header;
		self.in_for_header = outer_header || !applies_otherwise;
		let arg = self.parse_expr(argument_bp);
		self.in_for_header = outer_header;
		if arg == Empty && self.pos == start {
			return None; // an ø or [] written there is the argument: `x = sum []`
		}

		Some(Node::List(vec![lhs.clone(), arg], Bracket::None, Separator::Space))
	}
}

/// `id=1 kind="x"` in brackets: each attribute
fn attribute_items(index: &Node) -> Vec<Node> {
	match index.drop_meta() {
		Node::List(items, Bracket::None, _) => items.clone(),
		single => vec![single.clone()],
	}
}
