// P227 (user, 2026-10-09: "insist on braces to avoid errors"): a comma next to == or != without parentheses is an
// error asking for them, on either side; in brackets, call arguments and parenthesized it is fine
use crate::common::fails_with;
use crate::is;

#[test]
fn a_comma_next_to_a_comparison_needs_parentheses() {
	fails_with("(2 as float, 4.3 as int) == 2.0, 4", "a comma next to == needs parentheses");
	fails_with("x = (1, 2) == 1, 2; x", "write `(1, 2) == (1, 2)` or `((1, 2) == 1), 2`");
	fails_with("a=1; b=2; c=2; a, b == c", "write `(a, b) == c` or `a, (b == c)`");
	fails_with("a=1; a != 2, 3", "a comma next to != needs parentheses");
}

#[test]
fn a_parenthesized_comparison_beside_a_comma_is_fine() {
	is!("(2 as float, 4.3 as int) == (2.0, 4)", 1);
	is!("a=1; b=1; [a == b, 3]", warp::ints(vec![1, 3]));
	is!("f(a,b):=a; f(1 == 1, 2)", 1);
}
