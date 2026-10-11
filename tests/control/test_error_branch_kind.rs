//! An `error(…)` branch does not decide the kind of its if (it is the bottom kind): `if c then error("…") else 3` is an
//! Int, and taking the error branch raises the error through the runtime-error path, which `try` catches.
use crate::common::fails_with;
use crate::is;

#[test]
fn test_an_error_branch_keeps_the_other_branch_kind() {
	is!("x=3; (if x == 0 then error(\"zero\") else x) + 1", 4); // was wasm trap: cast failure
	is!("x=3; y = if x == 0 then error(\"zero\") else x; y + 1", 4);
	is!("x=3; (if x > 0 then x * 2 else error(\"negative\")) + 1", 7);
	is!("x=1.5; (if x == 0 then error(\"zero\") else x) + 1", 2.5);
}

#[test]
fn test_taking_the_error_branch_is_the_error() {
	fails_with("x=0; (if x == 0 then error(\"zero\") else x) + 1", "zero");
	fails_with("x=0; y = if x == 0 then error(\"zero\") else x; y + 1", "zero");
}

#[test]
fn test_else_if_chains_and_switch_defaults() {
	is!("x=2; (if x == 0 then error(\"zero\") else if x == 1 then 10 else 20) + 1", 21);
	fails_with("x=0; (if x == 1 then 10 else if x == 2 then 20 else error(\"no case\")) + 1", "no case");
	is!("c=2; y = switch c {1: 10 2: 20 default: error(\"no case\")}; y + 1", 21);
	fails_with("c=3; y = switch c {1: 10 2: 20 default: error(\"no case\")}; y + 1", "no case");
}

#[test]
fn test_try_catches_an_error_branch() {
	is!("f(x) := if x == 0 then error(\"zero\") else x; try y = f(0) else 7", 7);
	is!("f(x) := if x == 0 then error(\"zero\") else x; try y = f(2) else 7", 2);
}

#[test]
fn test_an_error_value_stays_a_value() {
	is!("x=error(\"bad\"); is_error(x)", 1);
}

// card braced-body: a declared result converts each branch, an `error(…)` stays the error: `-> text` gave the text
// "(error 'e')", `int g` with an error branch "not an int"; a ternary's error branch is the bottom kind as an if's
#[test]
fn test_a_declared_result_passes_an_error_on() {
	fails_with("def f(x) -> text { error(\"e\") }; f(1)", "e");
	fails_with("def f(x) -> text { if x > 0 then \"pos\" else error(\"neg\") }; f(-1)", "neg");
	is!("def f(x) -> text { if x > 0 then \"pos\" else error(\"neg\") }; f(1)", "pos");
	fails_with("int g(x) = x > 0 ? x : error(\"neg\"); g(-1)", "neg");
	is!("int g(x) = x > 0 ? x : error(\"neg\"); g(3) + 1", 4);
	is!("def g(x) := x > 0 ? x : error(\"neg\"); g(3) + 1", 4); // was wasm trap: cast failure
}
