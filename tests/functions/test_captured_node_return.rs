// A nested function's return kind follows the kinds of what it captures from its enclosing function: a captured mean
// of floats is a float there too (card captured-node-return)
use crate::is;

#[test]
fn a_nested_function_returns_a_float_from_a_captured_float() {
	is!("v(xs) := { m = mean(xs); f = x => x - m; f(1) }; v([float(1), float(4)])", -1.5);
	is!("use list; variance([float(1), float(3)])", 1);
}
