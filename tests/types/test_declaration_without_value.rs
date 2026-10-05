// `real x;` declares x without a value: P28 (user, 2026-10-05) reads it as the zero value of its type (Go)
use warp::is;

#[test]
fn reading_a_declaration_without_value_is_an_error() {
	// P28: no longer an error, the zero value (test_declaration_zero_value.rs)
	is!("real x; x*x", 0);
	is!("int x; x=x+1; x", 1);
}

#[test]
fn a_declaration_assigned_before_reading_works() {
	is!("real x; x = 2; x*x", 4);
	is!("int n\nn = 3\nn+1", 4);
}
