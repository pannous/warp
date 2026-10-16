//! card used-module: a used module's word operator is its call form too, so lib/regex.warp declares matches once,
//! `infix operator matches(…) := std_pure(…)`: the call `matches(s, p)` joins operator_matches from the module
use crate::is;

#[test]
fn the_call_form_of_a_used_modules_operator_joins_it() {
	is!("use regex; matches(\"abc\", \"a.c\") and \"abc\" matches \"a.c\"", true);
	is!("import regex; regex.matches(\"abc\", \"a.c\")", true);
}

#[test]
fn a_function_the_program_names_wins() {
	is!("use regex; matches(s, p) := 7; matches(1, 2)", 7);
}
