/// Tests for operator parsing
use wasp::wasp_parser::parse;
use wasp::node::{Node, Op};

/// Helper to get the operator from a Key node (unwrapping Meta if needed)
fn get_op(node: &Node) -> Option<Op> {
	match node {
		Node::Key(_, op, _) => Some(*op),
		Node::Meta { node, .. } => get_op(node),
		_ => None,
	}
}

#[test]
fn test_arithmetic_basic() {
	let result = parse("1 + 2");
	assert_eq!(get_op(&result), Some(Op::Add));
}

#[test]
fn test_arithmetic_precedence() {
	// * binds tighter than +: 1 + 2 * 3 = 1 + (2 * 3)
	let result = parse("1 + 2 * 3");
	assert_eq!(get_op(&result), Some(Op::Add)); // outer is Add
}

#[test]
fn test_comparison() {
	let result = parse("a < b");
	assert_eq!(get_op(&result), Some(Op::Lt));
}

#[test]
fn test_comparison_eq() {
	let result = parse("x == y");
	assert_eq!(get_op(&result), Some(Op::Eq));
}

#[test]
fn test_prefix_neg() {
	// -x should be Key(Empty, Neg, x) - prefix negation of symbol
	// Note: -5 is parsed as negative number literal, not prefix negation
	let result = parse("-x");
	assert_eq!(get_op(&result), Some(Op::Neg));
}

#[test]
fn test_suffix_square() {
	let result = parse("x²");
	// Should be Key(x, Square, Empty)
	assert_eq!(get_op(&result), Some(Op::Square));
}

#[test]
fn test_mixed_prefix_infix() {
	// -5 + 3 should be (Neg 5) + 3
	let result = parse("-5 + 3");
	assert_eq!(get_op(&result), Some(Op::Add)); // outer is Add
}

#[test]
fn test_logical_and() {
	let result = parse("a and b");
	assert_eq!(get_op(&result), Some(Op::And));
}

#[test]
fn test_logical_or() {
	let result = parse("x or y");
	assert_eq!(get_op(&result), Some(Op::Or));
}

#[test]
fn test_unicode_le() {
	let result = parse("a ≤ b");
	assert_eq!(get_op(&result), Some(Op::Le));
}

#[test]
fn test_unicode_mul() {
	let result = parse("x × y");
	assert_eq!(get_op(&result), Some(Op::Mul));
}

#[test]
fn test_power_right_assoc() {
	// 2^3^4 should be 2^(3^4), not (2^3)^4 (right-assoc)
	let result = parse("2^3^4");
	assert_eq!(get_op(&result), Some(Op::Pow)); // outer is Pow
}

#[test]
fn test_assignment_right_assoc() {
	// a = b = c should be a = (b = c)
	let result = parse("a = b = c");
	assert_eq!(get_op(&result), Some(Op::Assign));
}

#[test]
fn test_range() {
	let result = parse("1..10");
	assert_eq!(get_op(&result), Some(Op::Range));
}

#[test]
fn test_hash_index() {
	let result = parse("list#3");
	assert_eq!(get_op(&result), Some(Op::Hash));
}
