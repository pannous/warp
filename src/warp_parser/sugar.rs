//! The sugar pass (card rust-cleanup S1, notes/rust_cleanup.md; warp twin lib/compiler/sugar.warp): the parser
//! builds what the source says, `xs[i]`, `xs#(a..b)`, `a /! b`, `a /!= b`, `x => y`; parse() rewrites each written
//! form, one arm per form, into the tree the pipeline reads
use super::*;

/// The tree with every written form rewritten, deepest first
pub(crate) fn sugared(node: Node) -> Node {
	let node = match node {
		Node::Type { name, body } => Node::Type { name, body: Box::new(sugared(*body)) },
		other => crate::node::children_rewritten(other, sugared),
	};
	match node {
		Node::Key(target, Op::Subscript, index) => subscript(*target, *index),
		Node::Key(target, Op::Hash, index) if hash_slice_bounds(&index).is_some() => {
			let (start, end) = hash_slice_bounds(&index).expect("checked");
			call(SLICE_WORD, vec![*target, start, end])
		}
		Node::Key(left, Op::FloorDiv, right) => call(FLOOR_QUOTIENT, vec![*left, *right]),
		Node::Key(left, Op::FloorDivAssign, right) => {
			let quotient = call(FLOOR_QUOTIENT, vec![left.as_ref().clone(), *right]);
			Node::Key(left, Op::Assign, Box::new(quotient))
		}
		// `=>` reads as in JS while parsing (Op::FatArrow: its binding, a lambda's body, a field holding a lambda);
		// the program has one arrow, `->` (user 2026-10-14: "equal for us, but not for others")
		Node::Key(parameters, Op::FatArrow, body) => Node::Key(parameters, Op::Arrow, body),
		other => other,
	}
}
