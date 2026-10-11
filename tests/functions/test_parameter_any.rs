// card parameter-takes: a parameter that takes any value is annotated `any`; a parameter called with two kinds takes
// any value too (P173)
use crate::is;
use warp::warp_parser::parse;

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

/// arithmetic of an any value is a number decided at run time; joined with a text it takes its text form, as a known
/// number does (card server-def-any: `server def doubled(n)` takes n:any, and `"n " + doubled(n)` was "text + data")
#[test]
fn an_any_result_joins_a_text() {
	is!("doubled(x:any) := x * 2; \"n \" + doubled(1)", "n 2");
	is!("doubled(x:any) := x * 2; n = 1; n = doubled(n); \"n \" + n", "n 2");
	is!("half(x:any) := x / 2.0; half(3) + \" left\"", "1.5 left");
}

/// card recursive-sum: a parameter whose kind the body tests (`x is list`) takes any value; the else branch holds what
/// is no list (was "int + list": the list argument made x a list everywhere)
#[test]
fn a_parameter_tested_for_its_kind_takes_any() {
	is!("def total(x) = if x is list then sum(x.map(y => total(y))) else x; total([1 [2 3] [4 [5]]])", 15);
	is!("def total(x){ if x is list { s=0; for y in x { s += total(y) }; return s }; x }; total([1 [2 3]])", 6);
	is!("def depth(x) = if x is list then 1 + max(x.map(depth)) else 0; depth([1 [2 [3]]])", 3);
}
