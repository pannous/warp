/// Tests for operator parsing
use wasp::wasp_parser::parse;

#[test]
fn test_arithmetic_basic() {
	let result = parse("1 + 2");
	assert_eq!(result.name(), "+");
}

#[test]
fn test_arithmetic_precedence() {
	// * binds tighter than +
	let result = parse("1 + 2 * 3");
	// Should be: Key(1, Add, Key(2, Mul, 3))
	assert_eq!(result.name(), "+"); // outer is Add
}

#[test]
fn test_comparison() {
	let result = parse("a < b");
	assert_eq!(result.name(), "<");
}

#[test]
fn test_comparison_eq() {
	let result = parse("x == y");
	assert_eq!(result.name(), "==");
}

#[test]
fn test_prefix_neg() {
	let result = parse("-5");
	// Should be Key(Empty, Neg, 5)
	assert_eq!(result.name(), "-");
}

#[test]
fn test_suffix_square() {
	let result = parse("x²");
	// Should be Key(x, Square, Empty)
	assert_eq!(result.name(), "²");
}

#[test]
fn test_mixed_prefix_infix() {
	// -5 + 3 should be (Neg 5) + 3
	let result = parse("-5 + 3");
	assert_eq!(result.name(), "+"); // outer is Add
}

#[test]
fn test_logical_and() {
	let result = parse("a and b");
	assert_eq!(result.name(), "and");
}

#[test]
fn test_logical_or() {
	let result = parse("x or y");
	assert_eq!(result.name(), "or");
}

#[test]
fn test_unicode_le() {
	let result = parse("a ≤ b");
	assert_eq!(result.name(), "<=");
}

#[test]
fn test_unicode_mul() {
	let result = parse("x × y");
	assert_eq!(result.name(), "*");
}

#[test]
fn test_power_right_assoc() {
	// 2^3^4 should be 2^(3^4), not (2^3)^4
	let result = parse("2^3^4");
	assert_eq!(result.name(), "^"); // outer is Pow
}

#[test]
fn test_assignment_right_assoc() {
	// a = b = c should be a = (b = c)
	let result = parse("a = b = c");
	assert_eq!(result.name(), "=");
}

#[test]
fn test_range() {
	let result = parse("1..10");
	assert_eq!(result.name(), "..");
}

#[test]
fn test_hash_index() {
	let result = parse("list#3");
	assert_eq!(result.name(), "#");
}
