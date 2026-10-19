use warp::normalize::capture_hints;

const CODE: &str = "add(a, b) := a + b; add(2, 3)";
const EXPECTED_HINT: &str = "`add` on a list uses Warp's standard `add`; your `add` handles other values. Prefer a different name, such as `my_add`, if it should handle a list too.";

#[test]
fn a_library_word_hint_explains_which_definition_runs_and_what_to_prefer() {
	let (value, hints) = capture_hints(|| warp::wasm_emitter::eval(CODE));
	assert_eq!(value, 5);
	assert!(hints.iter().any(|hint| hint.reason == EXPECTED_HINT), "unexpected hints: {hints:?}");
}
