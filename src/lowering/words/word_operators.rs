//! Words for operators (word-choice rule: the canonical word plus aliases that work with a note): `root 4`,
//! `root(4)` and the pipe stage `2|square|root` (wiki/pipe.md) are sqrt; a variable or function the program names
//! root wins (P142). The other way round, the log glyphs are the log words (P226): `b⌞x` is `log(x, b)`, `x⌟` is
//! `ln(x)` and `x⌟b` is `log(x, b)`. `x↑n` and `x↓b` call their declarations, `operator_↑(x, n)` (card easy-add)

use super::nodes::{call, key};
use crate::node::Node;
use crate::operators::Op;
use crate::words::LOG_WORD;

const OPERATOR_ALIASES: [(&str, &str, Op); 1] = [("root", "sqrt", Op::Sqrt)];
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
		Node::Symbol(name) => aliases.iter().find(|(alias, _, _)| alias == name),
		_ => None,
	};
	let noted = |word: &Node| {
		let (alias, canonical, op) = alias_of(word).expect("guarded");
		crate::normalize::set_position_of(word);
		crate::normalize::hint(alias, canonical, &format!("{alias} is the word {canonical}"));
		*op
	};
	match node {
		// `root 4`, `root(4)`
		Node::List(items, _, _) if items.len() == 2 && alias_of(&items[0]).is_some() => {
			let op = noted(&items[0]);
			let [_, argument] = <[Node; 2]>::try_from(items).expect("two items");
			key(Node::Empty, op, operators(argument, aliases))
		}
		// the pipe stage `| root` (pipes.rs reads the operator alone)
		stage @ Node::Meta { .. } if crate::pipes::is_pipe_stage(&stage) => match stage {
			Node::Meta { node, data } if alias_of(&node).is_some() => {
				Node::Meta { node: Box::new(key(Node::Empty, noted(&node), Node::Empty)), data }
			}
			other => other.map_children(|child| operators(child, aliases)),
		},
		// the stage `then root` (P158, pipes.rs reads the operator alone)
		Node::Key(value, Op::Then, stage) if alias_of(&stage).is_some() => {
			let op = noted(&stage);
			key(operators(*value, aliases), Op::Then, key(Node::Empty, op, Node::Empty))
		}
		other => other.map_children(|child| operators(child, aliases)),
	}
}

fn log_calls(node: Node) -> Node {
	match node {
		Node::Key(base, Op::LogBase, value) => call(LOG_WORD, vec![log_calls(*value), log_calls(*base)]),
		Node::Key(value, Op::LogOf, base) if base.is_empty_node() => call(NATURAL_LOG, vec![log_calls(*value)]),
		Node::Key(value, Op::LogOf, base) => call(LOG_WORD, vec![log_calls(*value), log_calls(*base)]),
		// `2↑3`: an operator only a declaration gives meaning is the call of that declaration (lib/prelude.warp)
		Node::Key(left, op, right) if op.is_declared_only() => call(
			&format!("{}{}", crate::operators::OPERATOR_FUNCTION_PREFIX, op.as_str()),
			vec![log_calls(*left), log_calls(*right)],
		),
		other => other.map_children(log_calls),
	}
}
