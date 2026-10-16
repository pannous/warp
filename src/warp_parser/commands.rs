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

pub(super) fn is_print_word(node: &Node) -> bool {
	node.is_symbol(PRINT_WORD)
}

/// `print` or the call `print(…)`: a print statement starts here
pub(crate) fn starts_print(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) | Node::List(items, Bracket::None, Separator::Space) => {
			items.first().is_some_and(is_print_word)
		}
		other => is_print_word(other),
	}
}

/// Words separated by spaces form one expression: `upper "a"` is one argument of print
pub(super) fn one_expression(words: &[Node]) -> Node {
	match words {
		[single] => single.clone(),
		several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
	}
}

/// The call print(a, b, …): its arguments are separated by commas, like Python's
pub(crate) fn print_call(arguments: impl IntoIterator<Item = Node>) -> Node {
	Node::List([Symbol(PRINT_WORD.to_string())].into_iter().chain(arguments).collect(), Bracket::Round, Separator::None)
}

/// The arguments of `print(…)`: words separated by spaces (`print(upper "a")`) are one, else each item is one
pub(super) fn print_arguments(arguments: Node) -> Vec<Node> {
	match arguments.drop_meta() {
		Node::List(items, _, Separator::Space) if items.len() > 1 => vec![arguments],
		Node::List(items, _, _) => items.clone(),
		Node::Empty => vec![],
		_ => vec![arguments],
	}
}

/// The arguments of a parsed print list: the call print(a, b) has several, `print first xs` the one expression `first xs`
pub(crate) fn print_arguments_of(call: &[Node], bracket: &Bracket) -> Vec<Node> {
	match bracket {
		Bracket::Round => call[1..].to_vec(),
		_ => vec![one_expression(&call[1..])],
	}
}

/// A list as grouped by its separators, with the braceless print forms made explicit:
/// `print first xs` prints the one expression `first xs`, and in `print a, b` the comma binds looser than the space,
/// so the comma list [[print a], b] becomes the call print(a, b)
pub(super) fn grouped_list(items: Vec<Node>, bracket: Bracket, separator: Separator) -> Node {
	let items = if separator == Separator::Space { with_arrow_bodies(items) } else { items };
	let items = match separator {
		Separator::Space if bracket != Bracket::Square => {
			guarded_command(&items).map_or(items, |guarded| vec![guarded])
		}
		_ => items,
	};
	// `{ print upper n }`: a block of the one statement prints the one expression too
	let is_print_block =
		bracket == Bracket::Curly && separator == Separator::Space && items.len() > 2 && is_print_word(&items[0]);
	if is_print_block {
		return Node::List(vec![items[0].clone(), one_expression(&items[1..])], bracket, separator);
	}
	// `{ print i, i*2 }`: the block of the one call print(i, i*2)
	if let (Bracket::Curly, Separator::Comma, Some(Node::List(head, Bracket::None, Separator::Space))) =
		(&bracket, &separator, items.first().map(Node::drop_meta))
	{
		if is_print_word(&head[0]) {
			return Node::List(
				vec![print_call(head[1..].iter().chain(&items[1..]).cloned())],
				bracket,
				Separator::None,
			);
		}
	}
	if bracket != Bracket::None || !matches!(separator, Separator::Space | Separator::Comma) {
		return Node::List(items, bracket, separator);
	}
	match (&separator, items[0].drop_meta()) {
		(Separator::Space, head) if starts_print(head) && items.iter().skip(2).any(starts_print) => {
			let second = items.iter().skip(2).find(|item| starts_print(item)).expect("guarded");
			Diagnostic::at(second, TWO_STATEMENTS_ON_ONE_LINE).into_error()
		}
		(Separator::Space, head) if is_print_word(head) && items.len() > 2 => {
			Node::List(vec![items[0].clone(), one_expression(&items[1..])], bracket, separator)
		}
		(Separator::Comma, Node::List(head, Bracket::None, Separator::Space)) if is_print_word(&head[0]) => {
			print_call(head[1..].iter().chain(&items[1..]).cloned())
		}
		_ => Node::List(items, bracket, separator),
	}
}

/// A braceless call `sleep 1s`, `f 3`: a word applied to what follows it
pub(super) fn is_command(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, Bracket::None, Separator::Space) if items.len() > 1 && matches!(items[0].drop_meta(), Symbol(_)))
}
