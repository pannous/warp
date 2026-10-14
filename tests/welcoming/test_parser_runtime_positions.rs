//! card runtime-parser: a parser error and a run-time error name the source position they come from
use crate::common::fails_with;

#[test]
fn an_unexpected_character_names_its_position() {
	fails_with("x = 1\ny = 2 ¶ 3", "Unexpected character '¶' at 2:7");
}

#[test]
fn an_index_out_of_range_names_its_line() {
	fails_with("x = 1\nxs = [1, 2]\ny = xs#5\nprint y", "index out of range: 5 not in 1…2 at 3:");
}

#[test]
fn an_index_after_a_call_names_the_caller_line() {
	fails_with(
		"def f(n){\n  z = n\n  z\n}\nxs = [1, 2]\ny = f(1) + xs#5\nprint y",
		"index out of range: 5 not in 1…2 at 6:",
	);
}

#[test]
fn an_index_in_a_function_names_its_line() {
	fails_with("def f(xs){\n  z = 1\n  xs#5\n}\nprint f([1, 2])", "index out of range: 5 not in 1…2 at 3:");
}
