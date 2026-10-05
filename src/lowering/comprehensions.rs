//! `[x * x for x in 1..10]` and `[x for x in xs if x % 2 == 0]`: a list built by a loop,
//! `(made = []; for x in xs { if c { made.push(x * x) } }; made)`. Lowered first, so the loop and the push go through
//! every later pass like written ones.

use crate::library_words::substitute;
use crate::node::{Bracket, Node};
use crate::wasp_parser::parse;
use std::cell::Cell;

const FOR_WORD: &str = "for";
const IN_WORD: &str = "in";
const IF_WORD: &str = "if";
const MADE: &str = "comprehension_list";
const VARIABLE: &str = "comprehension_variable";
const SEQUENCE: &str = "comprehension_sequence";
const CONDITION: &str = "comprehension_condition";
const ELEMENT: &str = "comprehension_element";

pub fn lower(node: Node) -> Node {
	Lowering { count: Cell::new(0) }.lower(node)
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
		let number = self.count.get();
		self.count.set(number + 1);
		let made = format!("{MADE}_{number}");
		let push = format!("{made}.push({ELEMENT})");
		let body = if condition.is_some() { format!("if {CONDITION} {{ {push} }}") } else { push };
		let template = format!("({made} = []; for {VARIABLE} in {SEQUENCE} {{ {body} }}; {made})");
		let mut program = parse(&template);
		for (placeholder, replacement) in [(VARIABLE, variable), (SEQUENCE, sequence), (ELEMENT, element)] {
			program = substitute(program, placeholder, replacement);
		}
		if let Some(condition) = condition {
			program = substitute(program, CONDITION, condition);
		}
		Some(program)
	}
}
