//! Words for operators (word-choice rule: the canonical word plus aliases that work with a note): `root 4`,
//! `root(4)` and the pipe stage `2|square|root` (wiki/pipe.md) are sqrt; a variable or function the program names
//! root wins (P142). The other way round, the log glyphs are the log words (P226): `b⌞x` is `log(x, b)`, `x⌟` is
//! `ln(x)` and `x⌟b` is `log(x, b)`. `x↑n` and `x↓b` call their declarations, `operator_↑(x, n)` (card easy-add).
//! A declared word operator is also its call form: `divides(3, 12)` calls `operator_divides` unless the program defines
//! a function divides (card declared-word)

use super::nodes::{call, called_name, key};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::LOG_WORD;
use std::collections::HashSet;

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

/// `divides(3, 12)` after `infix operator divides(d, n) := …` is `operator_divides(3, 12)`; a function the program
/// names divides wins. Runs once the used modules joined, so their operators have call forms too
pub fn lower_call_forms(node: Node) -> Node {
	let call_forms = call_forms(&node);
	if call_forms.is_empty() {
		return node;
	}
	operator_calls(node, &call_forms)
}

/// The same for the definitions of the standard modules, before the words of a module only other modules use are
/// renamed apart: shell's `matches(line, pattern)` of its `use regex` (card matches-call)
pub fn lower_call_forms_in(definitions: Vec<Node>) -> Vec<Node> {
	let call_forms = call_forms(&Node::List(definitions.clone(), Bracket::None, Separator::Newline));
	definitions.into_iter().map(|definition| operator_calls(definition, &call_forms)).collect()
}

/// `divides` of a defined `operator_divides` and no defined `divides`
fn call_forms(node: &Node) -> HashSet<String> {
	let defined = crate::library_words::defined_names(node);
	defined
		.iter()
		.filter_map(|name| name.strip_prefix(crate::operators::OPERATOR_FUNCTION_PREFIX))
		.filter(|word| word.starts_with(char::is_alphabetic) && !defined.contains(*word))
		.map(str::to_string)
		.collect()
}

fn operator_calls(node: Node, call_forms: &HashSet<String>) -> Node {
	match node {
		Node::List(mut items, bracket, separator)
			if called_name(&items, &bracket, &separator).is_some_and(|name| call_forms.contains(name)) =>
		{
			let name = called_name(&items, &bracket, &separator).expect("guarded").to_string();
			items[0] = Node::Symbol(format!("{}{name}", crate::operators::OPERATOR_FUNCTION_PREFIX));
			Node::List(items, bracket, separator).map_children(|child| operator_calls(child, call_forms))
		}
		other => other.map_children(|child| operator_calls(child, call_forms)),
	}
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
