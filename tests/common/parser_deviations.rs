//! The Rust parser's quirks that parser.warp does not copy (notes/self_hosting_dropped.md, section Deviations): the Rust
//! tree rewritten to the clean shape, so the parser twins compare at the level where both agree
use warp::{Bracket, Node, Separator};

/// The Rust tree with each listed quirk in its clean shape
pub fn clean_rust_tree(node: &Node) -> Node {
	let cleaned = match node {
		Node::Key(left, op, right) => Node::Key(Box::new(clean_rust_tree(left)), *op, Box::new(clean_rust_tree(right))),
		Node::List(items, bracket, separator) => {
			Node::List(items.iter().map(clean_rust_tree).collect(), bracket.clone(), separator.clone())
		}
		Node::Meta { node: inner, data } => Node::Meta { node: Box::new(clean_rust_tree(inner)), data: data.clone() },
		Node::Error(inner) => Node::Error(Box::new(clean_rust_tree(inner))),
		other => other.clone(),
	};
	definition_head_spread(&cleaned)
		.or_else(|| bare_return_alone(&cleaned))
		.or_else(|| colon_body_unwrapped(&cleaned))
		.unwrap_or(cleaned)
}

/// `f(x, y) { body }` has the head `f(x, y)` of a call, its parameters spread (deviation definition-head)
fn definition_head_spread(node: &Node) -> Option<Node> {
	let Node::List(parts, Bracket::Round, Separator::None) = node else { return None };
	let [head, body] = parts.as_slice() else { return None };
	if !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	let Node::List(call, Bracket::Round, Separator::None) = head.drop_meta() else { return None };
	let [name, parameters] = call.as_slice() else { return None };
	if !matches!(name.drop_meta(), Node::Symbol(_)) {
		return None;
	}
	let spread = match parameters.drop_meta() {
		Node::Empty => vec![],
		Node::List(items, Bracket::Round, _) => items.clone(),
		_ => return None,
	};
	let head = Node::List(std::iter::once(name.clone()).chain(spread).collect(), Bracket::Round, Separator::None);
	Some(Node::List(vec![head, body.clone()], Bracket::Round, Separator::None))
}

/// `return` alone is the word return, no phrase holding ø (deviation bare-return)
fn bare_return_alone(node: &Node) -> Option<Node> {
	let Node::List(parts, Bracket::None, Separator::Space) = node else { return None };
	let [word, value] = parts.as_slice() else { return None };
	(word.drop_meta().is_symbol("return") && matches!(value.drop_meta(), Node::Empty)).then(|| word.clone())
}

/// `for 1..3: print it` keeps its colon body bare, as `for i in xs: x` does (deviation colon-body); `3 times: x` keeps its
/// block, which tells the loop from `3 times "ab"`
fn colon_body_unwrapped(node: &Node) -> Option<Node> {
	let Node::List(parts, Bracket::None, Separator::Space) = node else { return None };
	let [first, second, body] = parts.as_slice() else { return None };
	if !first.drop_meta().is_symbol("for") {
		return None;
	}
	let Node::List(statements, Bracket::Curly, Separator::Semicolon) = body.drop_meta() else { return None };
	let [statement] = statements.as_slice() else { return None };
	Some(Node::List(vec![first.clone(), second.clone(), statement.clone()], Bracket::None, Separator::Space))
}
