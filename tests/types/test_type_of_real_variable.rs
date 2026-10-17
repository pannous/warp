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

#[test] // card float-says: a variable has the type of the exact real it holds, as the value has (`sqrt(2) is real`)
fn a_variable_holding_a_root_is_real_as_the_root_is() {
	is!("x = sqrt(2); type(x)", Node::Symbol("real".to_string()));
	is!("x = sqrt(2); x is float", 0);
	is!("sqrt(2) is float", 0);
	is!("x = sin(1); x is real", 1);
	is!("x = sqrt(4); type(x)", Node::Symbol("int".to_string()));
}
