// #21 (user 2026-10-03, "x=π stays real"): a variable holding π has the type of π.
use warp::*;

#[test]
fn a_variable_holding_pi_is_real() {
	is!("type(π)", Node::Symbol("real".to_string()));
	is!("x=π; type(x)", Node::Symbol("real".to_string()));
	is!("x=π; x is real", 1);
	is!("x=π; x*2 > 6", 1);
}
