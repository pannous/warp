//! The whole matched value in an arm (user, card match-binding-arm): the word `value` for any subject, the innermost
//! match's in nested ones, unless the program names a variable value; `match name = expr { … }` assigns, then matches
use crate::is;

#[test]
fn value_is_the_matched_value() {
	is!("f(x) := [x, x+1]; match f(3) { [a, b] => #value + a; _ => 0 }", 5);
	is!("match 3 { n if value > 2 => value * 10; _ => 0 }", 30);
	is!("match 3 { n => match 5 { _ => value + n } }", 8);
}

#[test]
fn match_on_a_literal_in_a_body_or_an_arm() {
	is!("g(v) := match 1 { _ => v }; g(4)", 4);
	is!("match 3 { 3 => match 5 { 5 => 1; _ => 2 }; _ => 0 }", 1);
}

#[test]
fn a_variable_named_value_wins() {
	is!("value = 9; match [1, 2] { [a, b] => value; _ => 0 }", 9);
	is!("g(value) := match 1 { _ => value }; g(4)", 4);
}

#[test]
fn match_names_its_subject() {
	is!("f(x) := [x, x+1]; match alle = f(3) { [a, b] => #alle + b; _ => 0 }", 6);
	is!("f(x) := [x, x+1]; r = match alle=f(3) { [a, b] => #alle; _ => 0 }; r", 2);
	is!("f(x) := [x, x+1]; match (alle=f(3)) { [a, b] => #alle; _ => 0 }", 2);
	is!("f(x) := [x, x+1]; match alle=f(3) { _ => 0 }; alle[1]", 4);
}
