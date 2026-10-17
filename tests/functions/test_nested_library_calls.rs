//! card broadcast-library: a library word of a typed value nests inside another's parentheses as a user function does,
//! `round(floor 2.5)` is `round(floor(2.5))`, not round_to(floor, 2.5); a broadcast's result assigned is a list
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_library_word_nests_inside_another() {
	assert_eq!(printed("round(floor 2.5)"), "2");
	assert_eq!(printed("upper(lower \"A\")"), "\"A\"");
	assert_eq!(printed("first(sorted [3 1 2])"), "1");
	assert_eq!(printed("round(floor [1.5 2.5])"), "[1 2]");
}

#[test]
fn round_with_places_stays_round_to() {
	assert_eq!(printed("round(3.14159, 2)"), "3.14");
	assert_eq!(printed("round 3.14159 2"), "3.14");
}

#[test]
fn a_broadcast_result_is_a_list_variable() {
	assert_eq!(printed("x = floor [1.5 2.5]; round(x)"), "[1 2]");
	assert_eq!(printed("x = floor 2.5; round(x)"), "2");
}
