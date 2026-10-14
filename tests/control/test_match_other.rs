//! `other` and `else` catch the cases nothing else matches (user, matching roadmap), like `_` and `default`; a variable named
//! other stays a variable as the subject or in a guard
use crate::is;

#[test]
fn other_catches_the_rest() {
	is!("match 3 { 1 => \"one\"; other => \"many\" }", "many");
	is!("switch 1 { 1: \"one\"; other: \"many\" }", "one");
	is!("enum Coin { Penny, Dime }; c = Dime; match c { Penny: 1; other: 0 }", 0);
}

#[test]
fn else_catches_the_rest() {
	is!("fib(n) := match n { 0: 0; 1: 1; else: fib(n - 1) + fib(n - 2) }; fib(10)", 55);
	is!("match 3 { 1 => \"one\"; else => \"many\" }", "many");
}

#[test]
fn a_variable_named_other_stays_a_variable() {
	is!("other = 2; match other { 2 => \"two\"; _ => \"no\" }", "two");
	is!("other = 5; match 7 { n if n > other => \"more\"; _ => \"less\" }", "more");
}
