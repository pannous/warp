// `real x;` declares x without a value: reading it before an assignment is a loud error, never a silent zero
use crate::common::fails_with;
use warp::is;

#[test]
fn reading_a_declaration_without_value_is_an_error() {
	fails_with("real x; x*x", "x is declared without a value");
	fails_with("int x; x=x+1; x", "x is declared without a value");
}

#[test]
fn a_declaration_assigned_before_reading_works() {
	is!("real x; x = 2; x*x", 4);
	is!("int n\nn = 3\nn+1", 4);
}
