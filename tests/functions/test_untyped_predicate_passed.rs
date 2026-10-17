//! card untyped-passed: an untyped predicate `f(c) := is_digit(c) or c == '-'` passed as a value compares its
//! codepoint parameter in either order of the operands (it once said 'not comparable')
use crate::is;

#[test]
fn an_untyped_predicate_passed_as_a_value() {
	for predicate in ["is_digit(c) or c == '-'", "c == '-' or is_digit(c)"] {
		is!(&format!("use text\nf(c) := {predicate}\nc = Cursor(\"12-3ab\")\nc.take_while(f)"), "12-3");
	}
	is!("f(c) := is_digit(c) or c == '-'\napply(g, v) := g(v)\napply(f, '-')", true);
}
