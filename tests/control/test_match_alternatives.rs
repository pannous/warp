//! Alternatives in a match arm (card match-alternatives): `1 | 2 =>`, `1 or 2 =>` and `1, 2 =>` match either value;
//! `|` was the logical or, so the subject was compared with the value of `1 | 2` and the arm silently missed
use crate::common::fails_with;
use crate::is;

#[test]
fn bar_and_or_are_alternatives() {
	is!("match 2 { 1 | 2 => \"small\"; _ => \"big\" }", "small");
	is!("match 3 { 1 or 2 => \"small\"; _ => \"big\" }", "big");
	is!("x = \"b\"; match x { \"a\" | \"b\" | \"c\" => 1; _ => 2 }", 1);
	is!("switch 1 { 1 | 2: \"small\"; default: \"big\" }", "small");
}

#[test]
fn commas_are_alternatives() {
	is!("match 2 { 1, 2 => \"small\"; _ => \"big\" }", "small");
	is!("match 1 { 1, 2 => \"small\" }", "small");
	is!("match 5 { 1, 2 => \"small\"; 4, 5, 6 => \"middle\"; _ => \"big\" }", "middle");
}

#[test]
fn alternatives_of_list_patterns() {
	is!("match [2] { [1] | [2] => \"one item\"; _ => \"other\" }", "one item");
	is!("match [3, 4] { [] | [_] => 0; [a, b] => a + b }", 7);
}

#[test]
fn a_name_bound_in_an_alternative_is_an_error() {
	fails_with("match [1] { [a] | [a, _] => a; _ => 0 }", "alternatives bind no names");
}
