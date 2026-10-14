//! `parse(text)` is the code the text holds, as data (card parse-word, notes/parse_word.md): the same Node `data (…)`
//! gives, so `parse("$a + 0") == data ($a + 0)`. A constant text folds here; any other text is parsed at run time by
//! the host word parse_code (card parse-runtime). A program's own `parse` stays its own, and so does a method
//! `x.parse(…)`.

use super::blocks::DATA_WORD;
use crate::host::PARSE_CODE;
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;

const PARSE_WORD: &str = "parse";

pub fn lower(program: Node) -> Node {
	let mut mentioned = false;
	program.visit(&mut |part| mentioned |= part.is_symbol(PARSE_WORD));
	if !mentioned || crate::library_words::defined_names(&program).contains(PARSE_WORD) {
		return program;
	}
	parsed(program)
}

/// The code a text holds, or the first parse failure; never a file (`parse("a.warp")` is the code `a.warp`)
pub fn code_of(source: &str) -> Result<Node, String> {
	let code = crate::warp_parser::WarpParser::parse(source);
	match code.first_error().map(Node::drop_meta) {
		Some(Node::Error(failure)) => Err(failure.text_value().unwrap_or_else(|| failure.serialize())),
		_ => Ok(code),
	}
}

fn parsed(node: Node) -> Node {
	if let Node::Key(receiver, Op::Dot, method) = node {
		return Node::Key(Box::new(parsed(*receiver)), Op::Dot, method);
	}
	let Some(argument) = parse_argument(&node).cloned() else {
		return crate::node::children_rewritten(node, parsed);
	};
	// a text that fails to parse fails at run time, where `try` catches it
	match argument.text_value().and_then(|source| code_of(&source).ok()) {
		// grouped: a bare `data e` left of an operator is no value yet (`(data e) == x` is)
		Some(code) => {
			let data = Node::List(vec![symbol(DATA_WORD), code], Bracket::None, Separator::Space);
			Node::List(vec![data], Bracket::Round, Separator::None)
		}
		None => Node::List(
			vec![symbol(PARSE_CODE), parsed(argument)],
			Bracket::Round,
			Separator::None,
		),
	}
}

/// `parse "…"`, `parse(t)`, `parse t`: what is parsed
fn parse_argument(node: &Node) -> Option<&Node> {
	let Node::List(items, Bracket::None | Bracket::Round, Separator::Space | Separator::None) = node.drop_meta() else {
		return None;
	};
	let [word, argument] = items.as_slice() else {
		return None;
	};
	if !word.is_symbol(PARSE_WORD) {
		return None;
	}
	Some(match argument.drop_meta() {
		Node::List(inner, Bracket::Round, _) if inner.len() == 1 => &inner[0],
		_ => argument,
	})
}
