//! Static units, stage 4: lists of quantities (one signature per list), list words over them, sqrt of quantities and
//! `${d}` interpolation
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn test_lists_of_quantities() {
	assert_eq!(shown("xs = [1 m, 2 m, 50 cm]; xs#2"), "200 cm");
	assert_eq!(shown("xs = [1 m, 2 m]; sum(xs)"), "3 m");
	assert_eq!(shown("xs = [1 km, 300 m]; max(xs)"), "1000 m");
	assert_eq!(shown("xs = [1 km, 300 m]; min(xs)"), "300 m");
	assert_eq!(shown("xs = [1 m, 2 m]; last(xs)"), "2 m");
	assert_eq!(shown("xs = [1 m, 2 m]; count(xs)"), "2");
	assert_eq!(shown("xs = [1 m, 2 m]; t = 0 m; for x in xs { t += x }; t"), "3 m");
	fails_with("xs = [1 m, 2 s]; xs#1", "DimensionError");
}

#[test]
fn test_sqrt_halves_the_powers() {
	assert_eq!(shown("a = 0 m²; for i in 1..2 { a += 9 m² }; √a"), "3 m");
	fails_with("v = 0 m; for i in 1..2 { v += 9 m }; √v", "DimensionError");
}

#[test]
fn test_curly_interpolation_shows_the_unit() {
	assert_eq!(shown("d = 0 m; for i in 1..3 { d += 2 m }; \"distance ${d}\""), "\"distance 4 m\"");
}
