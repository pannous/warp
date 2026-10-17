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
	colon_body_unwrapped(&cleaned).unwrap_or(cleaned)
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
