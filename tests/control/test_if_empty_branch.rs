//! A branch yielding ø makes the if a Node of run-time kind: the other branch's number stays a number
use warp::*;

#[test]
fn an_if_with_an_empty_branch_yields_empty_or_the_number() {
	is!("if 1>2 then 3 else ø", Empty);
	is!("if 1<2 then 3 else ø", 3);
	is!("if 1>2 {3} else {ø}", Empty);
	is!("x = if 1<2 then ø else 3; x", Empty);
	is!("x = if 1>2 then ø else 3; x + 1", 4);
	is!("f() := if 1<2 then ø else 3; f()", Empty);
}
