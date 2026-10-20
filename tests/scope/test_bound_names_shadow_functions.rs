//! A name a function's body assigns is its local there, whichever function of that name the program also has: its
//! own, or a used file's (card local-variable-named, found composing lib/compiler's stages into one program)
use crate::is;

#[test]
fn a_local_named_like_a_function_is_the_local() {
	is!("digits(a, b, c, d) := 1; f(t: text) := { digits = t; slice digits 1 ø }; f(\"007\")", "07");
	is!("digits(a, b, c, d) := 1; f(t: text) := { digits = t; digits[1..] }; f(\"007\")", "07");
}

#[test]
fn a_local_named_like_another_used_files_function_is_the_local() {
	is!(
		"use \"tests/fixtures/bound_names/lexing.warp\"; use \"tests/fixtures/bound_names/trimming.warp\"; trimmed(\"007\")",
		"7"
	);
}

#[test]
fn a_parameter_is_not_another_functions_local_of_that_name() {
	is!("f() := { payload = ø; 1 }; g(payload: int) := 2 + payload; f() + g(3)", 6);
}

#[test]
fn a_local_named_like_a_function_heads_a_list_as_its_value() {
	is!("held(a, suffix) := a; f(x) := { held = x; [held, 1] }; f(2)", warp::warp_parser::parse("[2 1]"));
}
