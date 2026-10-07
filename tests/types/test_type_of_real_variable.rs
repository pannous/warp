// #21 (user 2026-10-03, "x=π stays real"): a variable holding π has the type of π.
use crate::is;
use warp::*;

#[test]
fn a_variable_holding_pi_is_real() {
	is!("type(π)", Node::Symbol("real".to_string()));
	is!("x=π; type(x)", Node::Symbol("real".to_string()));
	is!("x=π; x is real", 1);
	is!("x=π; x*2 > 6", 1);
}

#[test] // card type-pi: `type(2 * pi)` was "list"
fn an_expression_of_pi_is_real() {
	let real = || Node::Symbol("real".to_string());
	is!("type(2 * pi)", real());
	is!("type(pi + 1)", real());
	is!("x = 2 * π; type(x)", real());
	is!("x = 2 * pi; x is real", 1);
	is!("type(pi / pi)", Node::Symbol("int".to_string()));
}
