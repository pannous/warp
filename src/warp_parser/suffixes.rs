//! Suffixes: suffix operators, bang and test words, implicit application

use super::*;

/// The operands a braceless call binds in: from the right side of `or` to the right side of `+`; a `*` operand keeps
/// `h*w int` the typed array `(h*w) int`
const OPERAND_APPLICATION_BPS: std::ops::RangeInclusive<u8> = Op::Or.binding_power().1..=Op::Add.binding_power().1;

impl WarpParser {
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

	/// The word at the cursor is a loop header's last: the body follows it (`for i in 1 to count xs { … }`, `… do …`,
	/// `…:` or an indented line), so it is the argument of the word before rather than that word's body; at the end of
	/// the line it is the braceless body of `for i in 1 to n body`
	fn word_ends_header(&mut self) -> bool {
		let start = self.mark();
		let ends = self.at_identifier_start() && self.parse_symbol().is_ok() && {
			self.skip_spaces();
			let indented_line_follows = self.current_char() == '\n' && matches!(self.peek_char(1), ' ' | '\t');
			self.at_to_header_end() || self.matches_keyword(DO_WORD) || indented_line_follows
		};
		self.rewind(start);
		ends
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
		// call of its own: `on change s` is one call with two arguments; in a loop header only its last word is an argument
		// (`for i in 1 to count xs { … }`, never the body of `for i in 1 to n body`), elsewhere a word that begins the next
		// item is no argument (word_begins_item)
		let in_operand = OPERAND_APPLICATION_BPS.contains(&min_bp)
			&& min_bp != argument_bp
			&& if self.in_for_header { self.word_ends_header() } else { !self.word_begins_item() };
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
