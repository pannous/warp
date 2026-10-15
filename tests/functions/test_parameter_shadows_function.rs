//! card stack-overflow: a parameter named like a function, compared with a bare function, is an error, not a compiler
//! stack overflow
use crate::common::fails_with;

#[test]
fn a_parameter_compared_with_a_bare_function_asks_for_its_argument() {
	fails_with("g(x) := x % 2; h(x) := x % 3; f(g) := g == h; f(5)", "h needs 1 argument");
}
