//! Words for operators (word-choice rule: the canonical word plus aliases that work with a note): `root 4`,
//! `root(4)` and the pipe stage `2|square|root` (wiki/pipe.md) are sqrt; a variable or function the program names
//! root wins (P142). The other way round, the log glyphs are the log words (P226): `b⌞x` is `log(x, b)`, `x⌟` is
//! `ln(x)` and `x⌟b` is `log(x, b)`

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const OPERATOR_ALIASES: [(&str, &str, Op); 1] = [("root", "sqrt", Op::Sqrt)];
const LOG: &str = "log";
const NATURAL_LOG: &str = "ln";

pub fn lower(node: Node) -> Node {
	let node = log_calls(node);
	let words = OPERATOR_ALIASES.map(|(alias, _, _)| alias);
	if !node.mentions_any(&words) {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	let aliases: Vec<_> = OPERATOR_ALIASES.into_iter().filter(|(alias, _, _)| !defined.contains(*alias)).collect();
	if aliases.is_empty() {
		return node;
	}
	operators(node, &aliases)
}

fn operators(node: Node, aliases: &[(&str, &str, Op)]) -> Node {
	let alias_of = |word: &Node| match word.drop_meta() {
		Node::Symbol(name) => aliases.iter().find(|(alias, _, _)| alias == name).map(|(alias, canonical, op)| {
			crate::normalize::set_position_of(word);
			crate::normalize::hint(alias, canonical, &format!("{alias} is the word {canonical}"));
			*op
		}),
		_ => None,
	};
	match node {
		// `root 4`, `root(4)`
		Node::List(items, bracket, separator) if items.len() == 2 && alias_of(&items[0]).is_some() => {
			let op = alias_of(&items[0]).expect("guarded");
			let [_, argument] = <[Node; 2]>::try_from(items).expect("two items");
			let _ = (bracket, separator);
			Node::Key(Box::new(Node::Empty), op, Box::new(operators(argument, aliases)))
		}
		// the pipe stage `| root` (pipes.rs reads the operator alone)
		Node::Meta { node, data } if crate::pipes::is_pipe_stage(&Node::Meta { node: node.clone(), data: data.clone() }) && alias_of(&node).is_some() => {
			let op = alias_of(&node).expect("guarded");
			Node::Meta { node: Box::new(Node::Key(Box::new(Node::Empty), op, Box::new(Node::Empty))), data }
		}
		// the stage `then root` (P158, pipes.rs reads the operator alone)
		Node::Key(value, Op::Then, stage) if alias_of(&stage).is_some() => {
			let op = alias_of(&stage).expect("guarded");
			Node::Key(Box::new(operators(*value, aliases)), Op::Then, Box::new(Node::Key(Box::new(Node::Empty), op, Box::new(Node::Empty))))
		}
		other => other.map_children(|child| operators(child, aliases)),
	}
}

fn log_calls(node: Node) -> Node {
	match node {
		Node::Key(base, Op::LogBase, value) => call(LOG, vec![log_calls(*value), log_calls(*base)]),
		Node::Key(value, Op::LogOf, base) if matches!(base.drop_meta(), Node::Empty) => call(NATURAL_LOG, vec![log_calls(*value)]),
		Node::Key(value, Op::LogOf, base) => call(LOG, vec![log_calls(*value), log_calls(*base)]),
		other => other.map_children(log_calls),
	}
}

fn call(name: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(name.to_string())], arguments].concat(), Bracket::Round, Separator::None)
}
