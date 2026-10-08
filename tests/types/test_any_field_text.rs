//! The text of a value whose kind is known only at run time (card text-arithmetic): a field of an object that mixes kinds
//! is any-typed, and so is arithmetic on it
use warp::wasm_emitter::eval;

const MIXED: &str = "p = {dist: 0, name: \"run\"}; for i in 1..3 { p.dist += 5 }; ";

fn shown(code: &str) -> String {
	eval(&format!("{MIXED}{code}")).serialize().trim().to_string()
}

#[test]
fn test_arithmetic_on_an_any_typed_field_has_a_text() {
	assert_eq!(shown("text_form(p.dist / 2)"), "\"5\"");
	assert_eq!(shown("text_form(p.dist + 1)"), "\"11\"");
	assert_eq!(shown("x = p.dist * 2; \"x=${x}\""), "\"x=20\"");
	assert_eq!(shown("str(p.dist / 4)"), "\"2.5\"");
}
