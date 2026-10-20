//! A branch yielding ø makes the if a Node of run-time kind: the other branch's number stays a number
use crate::is;
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

// `{print b}` is the call print(b), which gives ø (issue #18): the if is ø, not the number b ('not an int' trap)
#[test]
fn a_branch_printing_a_number_gives_empty() {
	is!("b = 3; if b > 0 {print b}; 5", 5);
	is!("f(b) := { if b > 0 then {print b}; 0 }; f(3); 5", 5);
}

// an element of a list the if gives is of the branch's list type, not an int ('not an int' in plain_enums subjects_in)
#[test]
fn an_element_of_a_list_from_an_if_keeps_its_type() {
	is!("n = 1; d = if n == 1 then [\"ab\", \"cd\"] else ø; k = d#2; k", "cd");
	is!("n = 1; d = if n == 1 then [\"ab\", \"cd\"] else [\"x\"]; k = d#2; k", "cd");
	is!("g(n: any, s: any) := { d = if n == 1 then [\"key\", \"KEY\"] else ø; s[d#1] = d#2; s }; g(1, {})#key", "KEY");
}
