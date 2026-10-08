//! Chained assignments `a = b = value` (cards chain-unchecked, chain-field) and `++` on a text (card inc-text),
//! each as the type model computes it (tests/types/test_type_model.rs)
use crate::common::fails_with;
use crate::is;

#[test]
fn a_chained_assignment_checks_every_declared_type() {
	fails_with("a: int = 0; a = b = 2.5; a", "a is declared int");
}

#[test]
fn a_chained_assignment_gives_every_name_the_value() {
	is!("a = b = 5; a + b", 10);
	is!("class P { x: int }; p = P(1); a = p.x = 5; a", 5);
	is!("class P { x: int }; p = P(1); a = p.x = 5; a + p.x", 10);
	is!("m = {k: 1}; a = m.k = 7; a", 7);
}

#[test]
fn increment_steps_a_text_as_plus_one_does() {
	is!("s = \"a\"; s++; s", "a1");
	is!("s = \"a\"; s += 1; s", "a1");
	is!("x = 2.5; x++; x", 3.5);
	is!("i = 2; i++; i", 3);
}
