//! Units under `*`, exponents and rates (wiki/unit.md, wiki_features row 21): `3 m * 2 m` is 6 m², `10 km/h * 2 h` is 20 km.
//! Products count in the finer unit like sums (Decided "unit sums use the finer unit"); mismatches are a DimensionError.
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_products_of_lengths_are_areas() {
	assert_eq!(shown("3 m * 2 m"), "6m²");
	assert_eq!(shown("1 m * 10 cm"), "1000cm²");
	assert_eq!(shown("2 m * 3 m * 4 m"), "24m³");
	assert_eq!(shown("2km²"), "2km²");
	assert_eq!(shown("3 m * 2 m == 60000 cm²"), "yes");
}

#[test]
fn test_powers_of_quantities() {
	assert_eq!(shown("(2 km)²"), "4km²");
	assert_eq!(shown("(2 km)^2"), "4km²");
	assert_eq!(shown("(3 m)³"), "27m³");
}

#[test]
fn test_areas_add_divide_and_compare() {
	assert_eq!(shown("2 m² + 1 m²"), "3m²");
	assert_eq!(shown("1 m² + 1 cm²"), "10001cm²");
	assert_eq!(shown("6 m² / 2 m"), "3m");
	assert_eq!(shown("6 m² / 2 m²"), "3");
	fails_with("1 m² + 1 m", "DimensionError");
}

#[test]
fn test_rates_times_durations() {
	assert_eq!(shown("10 km/h * 2 h"), "20km");
	assert_eq!(shown("2 h * 10 km/h"), "20km");
	assert_eq!(shown("10 km/h * 30 min"), "5km");
	assert_eq!(shown("x = 10 km / 2 h; x * 2 h"), "10km");
	assert_eq!(shown("10 km/h * 3"), "30km/h");
	assert_eq!(shown("10 km/h * 2 kg"), "20km·kg/h"); // composite units (supervisor task units-composite)
}

#[test]
fn test_mixed_dimensions_stay_loud() {
	fails_with("1 km + 1 s", "DimensionError");
	assert_eq!(shown("3 m * 2 kg"), "6m·kg"); // composite units (supervisor task units-composite)
}
