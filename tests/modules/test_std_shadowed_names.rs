//! A function's local named like a module word is the local (card libm-function): math's angle(x, y) beside
//! `angle = 0.5` in a function
use crate::is;

#[test]
fn a_local_named_like_a_module_word_is_the_local() {
	is!(
		"use math; def s(x) { angle = 0.5; cos(angle) }; s(1)",
		0.8775825618903728
	);
	is!("use math; def s() { angle = 0.25; sqrt(angle) }; s()", 0.5);
	// the function angle stays callable beside the local (angle(1, 0) is 0 exactly: the browser's atan2 differs in the
	// last bit elsewhere)
	is!("use math; def s() { angle = 0.5; angle * 2 }; s() + angle(1, 0)", 1.0);
}

#[test]
fn a_local_named_like_a_program_function_is_the_local() {
	is!(
		"f(x) := x + 1; g(n) := { f = [n]; f }; g(3)",
		warp::warp_parser::parse("[3]")
	);
}

/// cards local-variable, local-variable-named: a local in one function hides no library or module word in another
#[test]
fn a_local_in_one_function_leaves_the_word_to_the_others() {
	is!(
		"def f() { lower = \"abc\"; lower.length }; def g(w) := w.lower(); g(\"AB\")",
		"ab"
	);
	is!(
		"f() := { words = \"a b\"; count(words) }; g(t) := words(t); g(\"x y\") == [\"x\", \"y\"]",
		true
	);
	is!(
		"def f() { words = \"a b\"; count(words) }; def g(t) := words(t); count(g(\"x y z\")) + f()",
		6
	);
}
