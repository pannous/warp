//! card braceless-argument: a braceless argument takes a range for every callable, as at statement level (`sum 1..4`):
//! `y = sum 1..4`, `sqrt 4..9`, `r 4..9` and `y = r 4..9` all pass the range; comparisons still end the argument
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn a_braceless_argument_takes_a_range() {
	is!("y = sum 1..4; y", 6);
	is!("x = 1 + sum 1..4; x", 7);
	is!("r(x) := x * 2; y = r 1..3; y", eval("[2 4]"));
	is!("r(x) := x * 2; r 1..3", eval("[2 4]"));
	is!("sqrt 4..10", eval("sqrt(4..10)"));
	is!("y = sqrt 4..10; y", eval("sqrt(4..10)"));
}

#[test]
fn a_comparison_still_ends_the_argument() {
	is!("y = sum [1 2] == 3; y", true);
}
