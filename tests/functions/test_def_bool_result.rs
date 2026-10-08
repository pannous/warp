//! A function returning a comparison returns a bool in every definition form (card def-bool, found by the type model)
use warp::wasm_emitter::eval;

#[test]
fn every_definition_form_returns_a_comparison_as_bool() {
	for code in [
		"def ok(x){x<10}; ok(3)",
		"fn ok(x) { return x < 10 }; ok(3)",
		"def ok(x): x<10; ok(3)",
		"ok(x) := x < 10; ok(3)",
		"def ok(x: int){x<10}; ok(3)",
	] {
		assert_eq!(eval(code).serialize(), "yes", "{code}");
	}
}
