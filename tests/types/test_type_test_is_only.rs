// #30 (user 2026-10-03, "Only `is` tests types"): `3 is int` tests the type, `3 == int` compares a value with a type
// (false) and educates toward `is`.
use crate::is;
use warp::normalize::capture_hints;

#[test]
fn is_tests_types() {
	is!("3 is int", 1);
	is!("3 is rational", 1);
	is!("x=3; x is a number", 1);
}

#[test]
fn equals_with_a_type_word_does_not_type_test() {
	is!("3 == int", 0);
	is!("x=3; x == int", 0);
	is!("int=3; 3 == int", 1);
}

#[test]
fn equals_with_a_type_word_educates_toward_is() {
	let hints = capture_hints(|| warp::wasm_emitter::eval("3 == int")).1;
	assert!(hints.iter().any(|hint| hint.canonical == "3 is int"), "{hints:?}");
}
