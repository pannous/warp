//! card text-join: every quantity of a longer text join joins as its text, not only the one next to a text
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn two_quantities_join_one_text() {
	assert_eq!(shown("x = 2 km\n\"a: \" + x + \" or \" + x"), "\"a: 2km or 2km\"");
	assert_eq!(shown("x = 2 km\ny = 3 s\n\"\" + x + y"), "\"2km3s\"");
}

#[test]
fn a_sum_of_quantities_still_adds() {
	assert_eq!(shown("x = 2 km\ny = 500 m\nx + y"), "2500m"); // the finest written unit
}
