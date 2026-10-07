// card parameter-takes: a parameter that takes any value is annotated `any`; a parameter called with two kinds takes
// any value too (P173)
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn an_any_parameter_takes_every_kind() {
	is!("check(got:any, want:any) := got == want; [check(1, 1), check(\"ab\", \"ab\"), check(\"ab\", \"cd\")]", parse("[1 1 0]"));
}

/// P173 (user, 2026-10-07): a parameter called with two kinds takes any value, silently (was "annotate it, e.g. want:any")
#[test]
fn a_parameter_called_with_two_kinds_takes_any() {
	is!("check(got, want) := got == want; [check(1, 1), check(\"ab\", \"cd\")]", parse("[1 0]"));
}

/// a number and a text whose kinds are known only at run time join as when they are known: 1 + "x" is "1x" (was the
/// runtime error not a number)
#[test]
fn an_any_value_and_a_number_join_as_text() {
	is!("id(x:any) := x; 1 + id('x')", "1x");
	is!("id(x:any) := x; id(\"ab\") + 3", "ab3");
	is!("id(x:any) := x; 1 + id(2)", 3);
}
