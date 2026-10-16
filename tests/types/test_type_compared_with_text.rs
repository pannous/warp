// card type-text-compare: `type(x)` prints int, yet `type(x) == "int"` is no: a type never equals a text (decision
// type-equal), so the comparison hints the type word.
use crate::is;
use warp::normalize::capture_hints;

#[test]
fn type_compared_with_text_stays_no() {
	is!("x=3; type(x) == \"int\"", 0);
	is!("x=3; type(x) == int", 1);
	is!("x=3; \"\" + type(x) == \"int\"", 0); // a type joins as `class int` (decision type-text)
}

#[test]
fn type_compared_with_text_hints_the_type_word() {
	let hints = capture_hints(|| warp::wasm_emitter::eval("x=3; type(x) == \"int\"")).1;
	assert!(hints.iter().any(|hint| hint.canonical == "type(x) == int"), "{hints:?}");
}

#[test]
fn text_naming_no_type_gives_no_hint() {
	let hints = capture_hints(|| warp::wasm_emitter::eval("x=3; type(x) == \"hello\"")).1;
	assert!(hints.is_empty(), "{hints:?}");
}
