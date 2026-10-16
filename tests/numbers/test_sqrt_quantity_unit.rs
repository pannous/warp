//! card sqrt-quantity-unit: √ of a quantity keeps its unit, halving its powers, also where the whole program is answered
//! at compile time (numbers::real)
use crate::is;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn a_square_root_keeps_the_unit() {
	assert_eq!(shown("√(4 m²)"), "2m");
	assert_eq!(shown("a = 4 m²; √a"), "2m");
	assert_eq!(shown("y = √(4 m²); y + 1 m"), "3m");
	assert_eq!(shown("√(9 m²/s²)"), "3m/s");
	is!("√(4 m²) == 2 m", true);
}

#[test]
fn an_irrational_root_shows_as_a_float_with_the_unit() {
	assert_eq!(shown("√(2 m²)"), "1.4142135623730951m");
	assert_eq!(shown("√2"), "√2");
}
