// A text held in a variable converts to an Int at run time like its literal does (`"12" as int` is 12):
// optional sign, decimal digits; anything else is the runtime error "invalid number"
use warp::error;
use crate::is;

#[test]
fn a_text_variable_converts_to_int() {
	is!("x=\"12\"; x as int", 12);
	is!("x=\"-7\"; int(x)", -7);
	is!("x=\"+40\"; (x as int) + 2", 42);
	is!("xs=[\"12\", \"30\"]; (xs#1 as int) * (xs#2 as int)", 360);
}

#[test]
fn a_text_without_a_number_is_a_runtime_error() {
	is!("x=\"ab\"; x as int", error("invalid number"));
	is!("x=\"\"; x as int", error("invalid number"));
	is!("x=\"1x\"; x as int", error("invalid number"));
}

#[test]
fn a_text_beyond_i64_parses_unbounded() {
	is!("x=\"99999999999999999999\"; x as int", warp::Node::Number(warp::Number::from_bigint("99999999999999999999".parse().unwrap())));
	is!("x=\"-99999999999999999999\"; x as int + 1 < 0", 1);
}
