//! An unannotated parameter its body gives a value of another kind takes any value, as one called with two kinds does
//! (P173); the value was read as the call's kind: `n = "x"` gave 120, `n = [1]` gave 1 (card param-retype)
use crate::is;
use warp::warp_parser::parse;

#[test]
fn a_parameter_given_another_kind_in_its_body_holds_it() {
	is!("f(n) := { n = \"x\"; n }; f(5)", 'x');
	is!("f(n) := { n = \"xy\"; n }; f(5)", "xy");
	is!("f(n) := { n = [1]; n }; f(5)", parse("[1]"));
	is!("f(n) := { n = 2.5; n }; f(5)", 2.5);
	is!("f(n) := { if n > 3 { n = \"big\" }; n }; [f(5), f(1)]", parse("[\"big\" 1]"));
}
