// card class-method-named: a method may be named like a prefix operator word (norm, abs, sqrt, cbrt): `p.norm()` reads
// as the method, so it can be called; a function of that name still cannot (P141, test_builtin_clash.rs)
use crate::common::fails_with;
use crate::is;

#[test]
fn a_method_named_like_an_operator_word_is_called() {
	is!("class P{x:int; y:int; norm() := x*x+y*y}; P(1,2).norm()", 5);
	is!("class P{x:int; y:int; norm() := x*x+y*y}; p = P(1,2); p.norm()", 5);
	is!("class P{x:int; abs() := x * 10}; P(3).abs()", 30);
	is!("class P{x:int; fun cbrt() { x * 2 }}; P(3).cbrt()", 6);
}

#[test]
fn the_operator_word_stays_an_operator_elsewhere() {
	is!("class P{x:int; norm() := x}; abs(-3) + norm(-4)", 7);
	fails_with("norm(x) := 777; norm(4)", "norm is an operator");
	fails_with("abs(x) := 777; abs(4)", "abs is an operator");
}

#[test]
fn a_function_keyword_definition_of_an_operator_word_is_refused() {
	fails_with("def norm(x): 777\nnorm(4)", "norm is an operator");
	fails_with("fun sqrt(x) { 777 }; sqrt(4)", "sqrt is an operator");
}
