//! card type-says-symbol: a variable assigned a lambda is a function, so `type(f)` names its function type as for
//! `def f(x)`, not symbol
use crate::is;

#[test]
fn the_type_of_a_variable_holding_a_lambda_is_a_function() {
	is!("f = x => x*2; type(f)", "function: (x) -> int");
	is!("f := x => x*2; type(f)", "function: (x) -> int");
	is!("f = x => x*2; type(function f)", "function: (x) -> int");
}

#[test]
fn a_variable_holding_a_lambda_answers_the_reflection_words() {
	is!("f = x => x*2; f.name", "f");
	is!("f = (a, b) => a + b; f.params", warp::texts(vec!["a", "b"]));
}
