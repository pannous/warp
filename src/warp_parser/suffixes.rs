//! Suffixes: suffix operators, bang and test words, implicit application

use super::*;
use crate::warp_parser::context::Context;

/// Topic of the Ask about `f x as T`, which casts the argument (card parameter-named-number)
const ARGUMENT_CAST: &str = "argument-cast";
/// Operators that may follow a suffix `!` directly: `x!+1`, `x!*2` (`!=` is the inequality)
const INFIX_AFTER_BANG: [char; 9] = ['+', '-', '*', '/', '%', '^', '<', '>', ')'];
/// How tight a suffix `!` binds: below the index `#`, above `^` and arithmetic
const BANG_BP: u8 = Op::Hash.binding_power().0.midpoint(Op::Pow.binding_power().0);

/// The operands a braceless call binds in: from the right side of `or` to the right side of `+`; a `*` operand keeps
/// `h*w int` the typed array `(h*w) int`
/// Words whose subject is the glued call before them in an assigned value: `x = f(y) where it > 1`, `x = f(y) replace …`
const SUBJECT_PHRASE_WORDS: [&str; 2] =
	[crate::comprehensions::WHERE_KEYWORD, crate::lowering::tree_walks::REPLACE_WORD];
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
		let in_assignment_context = min_bp <= 60 || self.context.branch_bp == Some(min_bp); // Assignment r_bp is 59
		let lhs_is_callable = match lhs.drop_meta() {
			Node::Symbol(_) => true,
			Node::List(_, Bracket::None, _) => true, // Function call from implicit application
			// `x = f(y) where it > 1`: the assigned value is the whole phrase, as `x = xs where it > 1` (card replace-level);
			// any other word after a glued call is its own (`f = func(x int) int {…}`)
			Node::List(items, Bracket::Round, Separator::None) => {
				in_assignment_context
					&& items.first().is_some_and(|head| head.symbol_name().is_some())
					&& SUBJECT_PHRASE_WORDS.iter().any(|word| self.matches_keyword(word))
			}
			_ => false,
		};
		let lhs_is_defined_function = lhs.symbol_name().is_some_and(|name| self.functions.contains(name));
		// application binds tighter than the logical, comparison and additive operators: `1 + count xs` is
		// `1 + count(xs)` for any name (card count-xs); a `:` operand keeps `a: b c d` apart, and an argument takes no
		// call of its own: `on change s` is one call with two arguments; in a loop header only its last word is an argument
		// (`for i in 1 to count xs { … }`, never the body of `for i in 1 to n body`), elsewhere a word that begins the next
		// item is no argument (word_begins_item)
		let in_operand = OPERAND_APPLICATION_BPS.contains(&min_bp)
			&& min_bp != argument_bp
			&& if self.context.in_for_header { self.word_ends_header() } else { !self.word_begins_item() };
		let ch = self.current_char();
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
		if (min_bp == 0 && !takes_one_argument)
			|| min_bp > max_bp_for_application
			|| self.context.glued_pair_bp == Some(min_bp)
		{
			return None;
		}
		if !lhs_is_callable || !self.can_start_atom() || !should_apply || (ch == '{' && self.context.in_header()) {
			return None;
		}

		let start = self.pos;
		// `while i < count t {…}`: a block after an operand's argument is the body, as after a loop header's; so is a
		// block after any call's argument in a condition, `if exists profile {…}`
		let in_for_header = self.context.in_for_header || !applies_otherwise || self.context.equals_compares;
		let arg = self.within(Context { in_for_header, ..self.context }, |parser| {
			parser.labelled_argument(argument_bp).unwrap_or_else(|| parser.parse_expr(argument_bp))
		});
		if arg == Empty && self.pos == start {
			return None; // an ø or [] written there is the argument: `x = sum []`
		}
		let arg = self.cast_argument(lhs, arg, argument_bp);
		// `s + f 2 each: 3`: the labelled arguments after it are the call's too, as at statement level
		let mut call = vec![lhs.clone(), arg];
		while let Some(labelled) = self.labelled_argument(argument_bp) {
			call.push(labelled);
		}
		Some(Node::List(call, Bracket::None, Separator::Space))
	}

	/// `r = move 1 from: 10 to: 3`: a preposition with a colon and a blank after it labels the next argument, also where
	/// the call is an operand (decision labels-calls), as `move 1 from: 10` labels it as a statement
	fn labelled_argument(&mut self, argument_bp: u8) -> Option<Node> {
		let blanks = (0..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		let word = self.word_at(blanks);
		let width = blanks + word.chars().count();
		let labels = self.peek_char(width) == ':' && matches!(self.peek_char(width + 1), ' ' | '\t');
		if !labels || !crate::type_name_matching::is_label_word(&Node::Symbol(word.clone())) {
			return None;
		}
		self.advance_by(width + 1);
		self.skip_spaces();
		let value = self.parse_expr(argument_bp);
		Some(Node::Key(Box::new(Node::Symbol(word)), Op::Colon, Box::new(value)))
	}

	/// `o = f 2.2 as int`: the cast belongs to the argument, f(2.2 as int), as in the statement `f 2.2 as int` and as
	/// `o = f 2 + 1` is f(2 + 1) (card parameter-named-number, user decision B); arithmetic after it stays in the argument
	fn cast_argument(&mut self, function: &Node, argument: Node, argument_bp: u8) -> Node {
		self.skip_spaces_and_inline_comments();
		let Some((Op::As, width)) = self.peek_operator() else {
			return argument;
		};
		let (line, column) = self.get_position();
		self.advance_by(width);
		self.skip_whitespace();
		let target = self.parse_expr(Op::As.binding_power().1);
		let cast = Node::Key(Box::new(argument), Op::As, Box::new(target));
		match self.argument_cast_reading(function, &cast, line, column) {
			Ok(()) => self.continue_expr(cast, argument_bp),
			Err(unanswered) => unanswered,
		}
	}

	/// `f x as T` of a function of the program, whose variants may differ by parameter or by result type: a warning
	/// naming both explicit forms, f(x as T) taken
	pub(super) fn argument_cast_reading(
		&self,
		function: &Node,
		cast: &Node,
		line: usize,
		column: usize,
	) -> Result<(), Node> {
		let (Node::Symbol(name), Node::Key(argument, Op::As, target)) = (function.drop_meta(), cast.drop_meta()) else {
			return Ok(());
		};
		if !self.functions.contains(name) {
			return Ok(());
		}
		let (argument, target) = (crate::normalize::operand_text(argument), crate::normalize::operand_text(target));
		let written = format!("{name} {argument} as {target}");
		let question =
			format!("does `as {target}` cast the argument or the result of {name}? (warp casts the argument)");
		let readings = vec![
			reading("the argument", &format!("{name}({argument} as {target})")),
			reading("the result", &format!("({name} {argument}) as {target}")),
		];
		ask(&Ask::new(ARGUMENT_CAST, question, readings, Fallback::Warning).written(&written).at(line, column))
			.map(|_| ())
	}
}
