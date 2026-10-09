//! card print-km: `"a: " + x as km` parses `("a: " + x) as km`; a text has no unit, so `as` converts the last operand
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn a_conversion_after_a_text_join_converts_the_last_operand() {
	assert_eq!(shown("x = 1500 m\n\"a: \" + x as km"), "\"a: 1.5km\"");
	assert_eq!(shown("x = 2 km\n\"in m: \" + x as m"), "\"in m: 2000m\"");
}

#[test]
fn a_grouped_conversion_still_works() {
	assert_eq!(shown("x = 1500 m\n\"a: \" + (x as km)"), "\"a: 1.5km\"");
}
