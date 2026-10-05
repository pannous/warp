//! `6 m² as cm²` converts to a powered unit like `in`; `·` (and ⋅) multiplies, so a printed `6 m·kg` reads back
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_as_converts_to_a_powered_unit() {
	assert_eq!(shown("6 m² as cm²"), "60000 cm²");
	assert_eq!(shown("6 m² in cm²"), "60000 cm²");
	assert_eq!(shown("2 m³ as cm³"), "2000000 cm³");
}

#[test]
fn test_middle_dot_multiplies() {
	assert_eq!(shown("2 m·kg"), "2 m·kg");
	assert_eq!(shown("2 m·kg + 1 m·kg"), "3 m·kg");
	assert_eq!(shown("3·4"), "12");
	assert_eq!(shown("3⋅4"), "12");
}
