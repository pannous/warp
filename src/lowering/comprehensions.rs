//! `[x * x for x in 1..10]` and `[x for x in xs if x % 2 == 0]`: a list built by a loop,
//! `(made = []; for x in xs { if c { made.push(x * x) } }; made)`. Lowered first, so the loop and the push go through
//! every later pass like written ones. `xs where it > 1` filters like `[it for it in xs if it > 1]`.

use crate::library_words::substitute;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasp_parser::parse;
use std::cell::Cell;

const FOR_WORD: &str = "for";
const IN_WORD: &str = "in";
const IF_WORD: &str = "if";
const WHERE_WORD: &str = "where";
/// The element a `where` condition reads as `it`
const WHERE_ELEMENT: &str = "where·element";
const MADE: &str = "comprehension_list";
const VARIABLE: &str = "comprehension_variable";
const SEQUENCE: &str = "comprehension_sequence";
const CONDITION: &str = "comprehension_condition";
const ELEMENT: &str = "comprehension_element";

pub fn lower(node: Node) -> Node {
	Lowering { count: Cell::new(0) }.lower(node)
}

/// `xs where it > 1` as the comprehension `[it for it in xs if it > 1]`; before welcome_forms reads `xs where …` as
/// juxtaposed words. A condition not reading `it` is an error: it would filter nothing it can name.
pub fn lower_where(node: Node) -> Node {
	where_filters(where_reassociated(node))
}

fn where_filters(node: Node) -> Node {
	let node = node.map_children(where_filters);
	let Node::List(items, bracket, separator) = node else { return node };
	let Some(at) = where_position(&items) else { return Node::List(items, bracket, separator) };
	let filtered = where_comprehension(&items[at - 1], &items[at + 1]);
	// `xs where c` alone, or the last argument of a call `count(xs where c)`
	if at == 1 && bracket == Bracket::None {
		return filtered;
	}
	let mut items = items;
	items.truncate(at - 1);
	items.push(filtered);
	Node::List(items, bracket, separator)
}

/// `[it, (for it in xs if), condition]`, as the parser groups a comprehension
fn where_comprehension(subject: &Node, condition: &Node) -> Node {
	if !crate::lambdas::mentions(condition, crate::lambdas::IMPLICIT_PARAMETER) {
		let subject = subject.serialize();
		return crate::node::error(&format!("`{subject} where {}` filters by each element `it`: write {subject} where it > 1", condition.serialize()));
	}
	// a name of its own: in a function of one parameter `it` is that parameter
	let element = Node::Symbol(WHERE_ELEMENT.to_string());
	let condition = substitute(condition.clone(), crate::lambdas::IMPLICIT_PARAMETER, &element);
	let word = |word: &str| Node::Symbol(word.to_string());
	let clause = Node::List(vec![word(FOR_WORD), element.clone(), word(IN_WORD), subject.clone(), word(IF_WORD)], Bracket::None, Separator::Space);
	Node::List(vec![element, clause, condition], Bracket::Square, Separator::Space)
}

/// `xs where it > 1` parses as `(xs where it) > 1`: the operators right of `where` join its condition,
/// `[xs, where, it > 1]`, so the filter takes the whole condition (`it > 1 and it < 5` too)
fn where_reassociated(node: Node) -> Node {
	match where_flattened(node) {
		Node::Key(left, op, right) if !matches!(op, Op::Assign | Op::Define | Op::Colon) && !op.is_compound_assign() => {
			let (left, right) = (where_reassociated(*left), where_reassociated(*right));
			match left.drop_meta() {
				Node::List(items, bracket, separator) if where_position(items).is_some() => {
					let mut items = items.clone();
					let condition = items.pop().expect("where has a condition");
					items.push(Node::Key(Box::new(condition), op, Box::new(right)));
					Node::List(items, bracket.clone(), separator.clone())
				}
				_ => Node::Key(Box::new(left), op, Box::new(right)),
			}
		}
		other => other.map_children(where_reassociated),
	}
}

/// `[[xs, where], it]`, as the right side of an assignment parses, as `[xs, where, it]`
fn where_flattened(node: Node) -> Node {
	let Node::List(items, bracket, separator) = node else { return node };
	let Some(Node::List(inner, Bracket::None, _)) = items.first().map(Node::drop_meta) else { return Node::List(items, bracket, separator) };
	if !inner.last().is_some_and(|word| matches!(word.drop_meta(), Node::Symbol(symbol) if symbol == WHERE_WORD)) {
		return Node::List(items, bracket, separator);
	}
	let flat = inner.clone().into_iter().chain(items[1..].iter().cloned()).collect();
	Node::List(flat, bracket, separator)
}

/// The position of `where` in `[… subject, where, condition]`
fn where_position(items: &[Node]) -> Option<usize> {
	(items.len() >= 3).then(|| items.len() - 2).filter(|&at| matches!(items[at].drop_meta(), Node::Symbol(symbol) if symbol == WHERE_WORD))
}

struct Lowering {
	count: Cell<usize>,
}

impl Lowering {
	fn lower(&self, node: Node) -> Node {
		match node {
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.lower(item)).collect();
				match bracket {
					Bracket::Square => self.comprehension(&items).unwrap_or(Node::List(items, bracket, separator)),
					// Python's generator argument `sum(x * x for x in xs)`: the call of the comprehension's list
					Bracket::Round if separator == Separator::None && items.len() > 1 => match self.comprehension(&items[1..]) {
						Some(list) => Node::List(vec![items[0].clone(), list], bracket, separator),
						None => Node::List(items, bracket, separator),
					},
					_ => Node::List(items, bracket, separator),
				}
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.lower(*left)), op, Box::new(self.lower(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.lower(*node)), data },
			other => other,
		}
	}

	/// The items `[element, (for v in xs), …]` as the parser groups them; with a filter `(for v in xs if) condition`
	fn comprehension(&self, items: &[Node]) -> Option<Node> {
		let [element, clause, rest @ ..] = items else { return None };
		let Node::List(words, _, _) = clause.drop_meta() else { return None };
		let [for_word, variable, in_word, sequence, tail @ ..] = words.as_slice() else { return None };
		let is_word = |node: &Node, word: &str| matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == word);
		if !is_word(for_word, FOR_WORD) || !is_word(in_word, IN_WORD) || !matches!(variable.drop_meta(), Node::Symbol(_)) {
			return None;
		}
		let condition = match (tail, rest) {
			([], []) => None,
			([nothing], []) if matches!(nothing.drop_meta(), Node::Empty) => None,
			([if_word], [condition]) if is_word(if_word, IF_WORD) => Some(condition),
			_ => return None,
		};
		Some(self.built(variable, sequence, element, condition))
	}

	/// `(made = []; for variable in sequence { if condition { made.push(element) } }; made)`
	fn built(&self, variable: &Node, sequence: &Node, element: &Node, condition: Option<&Node>) -> Node {
		let number = self.count.get();
		self.count.set(number + 1);
		let made = format!("{MADE}_{number}");
		let push = format!("{made}.push({ELEMENT})");
		let body = if condition.is_some() { format!("if {CONDITION} {{ {push} }}") } else { push };
		let template = format!("(let {made} = []; for {VARIABLE} in {SEQUENCE} {{ {body} }}; {made})");
		let mut program = parse(&template);
		for (placeholder, replacement) in [(VARIABLE, variable), (SEQUENCE, sequence), (ELEMENT, element)] {
			program = substitute(program, placeholder, replacement);
		}
		if let Some(condition) = condition {
			program = substitute(program, CONDITION, condition);
		}
		program
	}
}
