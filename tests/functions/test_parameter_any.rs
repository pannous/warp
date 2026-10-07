// card parameter-takes: a parameter that takes any value is annotated `any`; a parameter called with two kinds says so
// in its error: "annotate it, e.g. want:any"
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn an_any_parameter_takes_every_kind() {
	is!("check(got:any, want:any) := got == want; [check(1, 1), check(\"ab\", \"ab\"), check(\"ab\", \"cd\")]", parse("[1 1 0]"));
}

#[test]
fn a_parameter_called_with_two_kinds_names_any() {
	let code = "check(got, want) := got == want; [check(1, 1), check(\"ab\", \"cd\")]";
	crate::common::fails_with(code, "annotate it, e.g. ");
	crate::common::fails_with(code, ":any");
}
