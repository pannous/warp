// `if c then s += 5`: a branch without braces takes the whole statement, an assignment or a compound assignment
// included (card if-parses: it parsed as `(if c then s) += 5`, and `(if c then s) = 5` silently changed nothing)
use crate::is;

#[test]
fn a_then_branch_takes_an_assignment() {
	is!("s=0; if 1 > 0 then s += 5; s", 5);
	is!("s=0; if 1 > 0 then s = 5; s", 5);
	is!("s=1; if 1 > 0 then s *= 3; s", 3);
	is!("s=0; if 1 < 0 then s += 5; s", 0);
}

#[test]
fn an_else_branch_takes_an_assignment() {
	is!("s=0; if 1 > 0 then s = 5 else s = 7; s", 5);
	is!("s=0; if 1 < 0 then s += 5 else s -= 2; s", -2);
	is!("s=0; for x in [1, 5, 3] { if x > 2 then s += x else s -= x }; s", 7);
}

#[test]
fn a_branch_value_is_still_a_value() {
	is!("x = if 1 > 0 then 3 else 4; x", 3);
	is!("x = if 1 < 0 then 3 else 4; x + 1", 5);
}
