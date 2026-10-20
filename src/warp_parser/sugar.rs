//! The sugar pass (card rust-cleanup S1, notes/rust_cleanup.md; warp twin lib/compiler/sugar.warp): the parser
//! builds what the source says, `xs[i]`, `xs#(a..b)`, `a /! b`, `a /!= b`, `x => y`; parse() rewrites each written
//! form, one arm per form, into the tree the pipeline reads
use super::*;

/// The tree with every written form rewritten, deepest first
pub(crate) fn sugared(node: Node) -> Node {
	rewritten(node, written_form)
}

/// The tree parser.warp gives (compiler/self_hosted.rs) with its `is`, `==` and `!=` read as parse() reads them
pub(crate) fn equalities_read(node: Node) -> Node {
	rewritten(node, read_equality)
}

fn rewritten(node: Node, form: fn(Node) -> Node) -> Node {
	let node = match node {
		Node::Type { name, body } => Node::Type { name, body: Box::new(rewritten(*body, form)) },
		other => crate::node::children_rewritten(other, |child| rewritten(child, form)),
	};
	form(node)
}

fn written_form(node: Node) -> Node {
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
		other => read_equality(other),
	}
}

/// `x is v` is `x == v`, which tests the type where v is a type word; `x == v` and `x != v` mark a type word v as the
/// type value compared (type_tests.rs equality_operand: only `is` tests a type, user decision #30)
fn read_equality(node: Node) -> Node {
	match node {
		Node::Key(left, Op::Is, right) => Node::Key(left, Op::Eq, right),
		Node::Key(left, op, right) if op.is_equality() => {
			let right = crate::type_tests::equality_operand(&left, *right);
			let left = crate::type_tests::equality_operand(&right, *left);
			Node::Key(Box::new(left), op, Box::new(right))
		}
		other => other,
	}
}
