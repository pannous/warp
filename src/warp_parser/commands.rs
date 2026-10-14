//! Commands: print and braceless calls `print x, y`, `sleep 1s`

use super::*;

impl WarpParser {
	/// The words up to the end of the line, one expression: the colon body `print i` of `for i in 1..3 : print i`
	pub(super) fn rest_of_statement(&mut self) -> Node {
		let words = self.words_of_expression();
		if words.len() < 2 || !is_print_word(&words[0]) || self.current_char() != ',' {
			return match words.len() {
				1 => words[0].clone(),
				_ => grouped_list(words, Bracket::None, Separator::Space), // `for c in s: print ord c` prints `ord c`
			};
		}
		// `for i in 1..3: print i, i*2` prints both
		self.print_call_from(one_expression(&words[1..]), Self::expression_of_words)
	}

	/// The words up to the end of the line, a comma or an `else` as one expression: `ord c` of `print ord c, 2`
	pub(super) fn expression_of_words(&mut self) -> Node {
		self.expression_binding(0)
	}

	pub(super) fn expression_binding(&mut self, min_bp: u8) -> Node {
		one_expression(&self.words_binding(min_bp))
	}

	/// The call print(first, …): the arguments after `first` follow commas, each read by `argument`
	pub(super) fn print_call_from(&mut self, first: Node, argument: impl Fn(&mut Self) -> Node) -> Node {
		let mut arguments = vec![first];
		while self.current_char() == ',' {
			self.advance();
			self.skip_blanks();
			arguments.push(argument(self));
		}
		print_call(arguments)
	}

	/// The words up to the end of the line, a comma or an `else`, `ord c` of `print ord c`
	pub(super) fn words_of_expression(&mut self) -> Vec<Node> {
		self.words_binding(0)
	}

	/// The words of an expression, each binding at least as tight as `min_bp`: `print 1` of `then print 1 else …`
	fn words_binding(&mut self, min_bp: u8) -> Vec<Node> {
		let mut words = vec![self.with_equals_comparing(false, |parser| parser.parse_expr(min_bp))];
		while self.braceless_argument_follows() && !self.matches_keyword("else") && self.at_else_if_word().is_none() {
			words.push(self.with_equals_comparing(false, |parser| parser.parse_expr(min_bp)));
		}
		words
	}

	/// `print x` without parentheses follows the blanks here: true with the word taken, the argument next
	pub(super) fn take_braceless_print(&mut self) -> bool {
		self.skip_blanks();
		let takes = self.matches_keyword(PRINT_WORD) && matches!(self.peek_char(PRINT_WORD.len()), ' ' | '\t');
		if takes {
			self.advance_by(PRINT_WORD.len());
		}
		takes && self.braceless_argument_follows()
	}

	/// An argument without parentheses follows the blanks here, on the same line
	pub(super) fn braceless_argument_follows(&mut self) -> bool {
		self.skip_blanks();
		self.can_start_atom()
	}
}
