//! Quantities hold exact rational amounts (reals.rs Rational): sums of rates and quotients that are no whole number of their
//! unit print exactly, no longer "not a whole number … yet"
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_non_whole_results_are_exact() {
	assert_eq!(shown("1 km/h + 1 m/s"), "23/18 m/s");
	assert_eq!(shown("10 km / 3 h"), "10/3 km/h");
	assert_eq!(shown("1 km/h in m/s"), "5/18 m/s");
	assert_eq!(shown("1 m / 4"), "1/4 m");
	assert_eq!(shown("1 m / 3 m"), "1/3");
}

#[test]
fn test_exact_amounts_compute_on() {
	assert_eq!(shown("(1 m / 3) * 3"), "1 m");
	assert_eq!(shown("90 minutes in hours"), "3/2 h");
	assert_eq!(shown("1 km/h < 1 m/s"), "1");
	assert_eq!(shown("150 cm in m"), "3/2 m");
}
