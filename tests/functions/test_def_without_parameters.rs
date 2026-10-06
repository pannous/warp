// `def test: body` defines a function without parameters (wiki/signal.md), like `def test() {body}`
use crate::is;

#[test]
fn a_def_without_parameters_is_a_function() {
	is!("def two: 1+1; two() * two()", 4);
	is!("def answer: 42; answer() + 1", 43);
}
