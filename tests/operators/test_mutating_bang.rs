// D2 round 3 (user 2026-10-03, "By position"): `fn!` after a function or method name mutates in place:
// `x.upper!` and `upper x!` assign the result back to x. `{…}!` and a lone `x!` keep evaluating.
use crate::is;

#[test]
fn a_method_with_bang_assigns_back() {
	is!("x=\"ab\"; x.upper!; x", "AB");
	is!("x=\"ab\"; x.upper!", "AB");
	is!("x=\"ab\"; x.upper()!; x", "AB");
	is!("xs=[3 1 2]; xs.sort!; xs#1", 1);
	is!("xs=[1 2 3]; xs.reverse!; xs#1", 3);
}

#[test]
fn a_function_with_bang_on_its_argument_assigns_back() {
	is!("x=\"ab\"; upper x!; x", "AB");
	is!("xs=[1 2 3]; reverse xs!; xs#1", 3);
	is!("x=\"ab\"; upper(x)!; x", "AB");
	is!("twice:=it*2; x=3; twice x!; x", 6);
	is!("twice:=it*2; x=3; twice(x)!; x", 6);
}

#[test]
fn without_bang_the_value_is_unchanged() {
	is!("x=\"ab\"; y=x.upper; x", "ab");
	is!("x=\"ab\"; y=upper x; x", "ab");
}

#[test]
fn bang_after_a_block_or_lone_name_evaluates() {
	is!("a=6; {a*a}!", 36);
	is!("f:={1+2}; f!", 3);
	is!("x=5; x!", 5);
}
