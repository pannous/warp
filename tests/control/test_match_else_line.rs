//! `else => …` on its own line is the catch-all arm of the match, not an else branch of the arm above (card match-arm)
use crate::is;

#[test]
fn else_arm_on_its_own_line() {
	is!("f(n) := match n {\n  1 => \"one\"\n  else => \"many\"\n}\nf(3)", "many");
	is!("f(n) := match n {\n  1 => \"one\"\n  else => \"many\"\n}\nf(1)", "one");
}

#[test]
fn else_branch_on_its_own_line_stays() {
	is!("x = 0\nif x then \"a\"\nelse \"b\"", "b");
}
