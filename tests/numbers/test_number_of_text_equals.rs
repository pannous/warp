//! card number-number: a number a run-time text spells equals a number literal by value, whatever kind the text gives
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn the_number_a_text_spells_equals_a_literal() {
	assert_eq!(printed("x = \"1.5\"; x as number == 1.5"), "yes");
	assert_eq!(printed("x = \"1.5\"; number(x) == 1.5"), "yes");
	assert_eq!(printed("x = \"2.5\"; x as number == 2"), "no");
	assert_eq!(printed("x = \"2.5\"; x as number != 2"), "yes");
	assert_eq!(printed("x = \"12\"; number(x) == 12"), "yes");
	assert_eq!(printed("x = \"12\"; x as number == 12.0"), "yes");
	assert_eq!(printed("x = \"1/2\"; x as number == 0.5"), "yes");
}
