// A duration or a date as a program's value reads as itself, not as the Rust type behind it
use warp::*;

#[test]
fn a_duration_and_a_date_have_their_text() {
	assert_eq!(wasm_emitter::eval("1 day + 3 hours").serialize(), "1 day 3 hours");
	assert_eq!(wasm_emitter::eval("90 minutes").serialize(), "1 hour 30 minutes");
	assert_eq!(wasm_emitter::eval("2024-01-31 + 1 day").serialize(), "2024-02-01");
}
