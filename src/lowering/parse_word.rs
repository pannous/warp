//! `parse(text)` of a constant text is the code the text holds, as data (card parse-word, notes/parse_word.md): the
//! same Node `data (…)` gives, so `parse("$a + 0") == data ($a + 0)`. A program's own `parse` stays its own.

use super::blocks::DATA_WORD;
use crate::node::{symbol, Bracket, Node, Separator};

const PARSE_WORD: &str = "parse";

pub fn lower(program: Node) -> Node {
	let mut mentioned = false;
	program.visit(&mut |part| mentioned |= part.is_symbol(PARSE_WORD));
	if !mentioned || super::welcome_forms::defined_names(&program).contains(PARSE_WORD) {
		return program;
	}
	parsed(program)
}

fn parsed(node: Node) -> Node {
	match constant_source(&node) {
		// grouped: a bare `data e` left of an operator is no value yet (`(data e) == x` is)
		Some(source) => Node::List(
			vec![Node::List(
				vec![symbol(DATA_WORD), crate::warp_parser::parse(&source)],
				Bracket::None,
				Separator::Space,
			)],
			Bracket::Round,
			Separator::None,
		),
		None => crate::node::children_rewritten(node, parsed),
	}
}

/// `parse "…"` / `parse("…")`: the text
fn constant_source(node: &Node) -> Option<String> {
	let Node::List(items, Bracket::None | Bracket::Round, Separator::Space | Separator::None) = node.drop_meta() else {
		return None;
	};
	let [word, argument] = items.as_slice() else {
		return None;
	};
	if !word.is_symbol(PARSE_WORD) {
		return None;
	}
	match argument.drop_meta() {
		Node::Text(source) => Some(source.clone()),
		Node::List(inner, Bracket::Round, _) if inner.len() == 1 => match inner[0].drop_meta() {
			Node::Text(source) => Some(source.clone()),
			_ => None,
		},
		_ => None,
	}
}
