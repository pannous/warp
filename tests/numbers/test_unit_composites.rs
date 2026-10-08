//! Composite units (wiki/unit.md): a quantity is an amount times units to powers, `km/h` is km·h⁻¹. Products of different
//! dimensions (`2 m * 3 kg` → 6 m·kg) and conversions of powered or composite units (`6 m² in cm²`, `36 km/h in m/s`).
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_products_of_different_dimensions() {
	assert_eq!(shown("2 m * 3 kg"), "6m·kg");
	assert_eq!(shown("kg*m/s²"), "1kg·m/s²");
	assert_eq!(shown("N = kg*m/s²; 2*N"), "2kg·m/s²");
	assert_eq!(shown("10 km/h * 2 kg"), "20km·kg/h");
	assert_eq!(shown("2 m * 3 kg / 3 kg"), "2m");
}

#[test]
fn test_converting_powered_and_composite_units() {
	assert_eq!(shown("6 m² in cm²"), "60000cm²");
	assert_eq!(shown("36 km/h in m/s"), "10m/s");
	assert_eq!(shown("150 cm in m"), "1.5m");
	fails_with("6 m² in cm", "DimensionError");
	fails_with("1 km/h in kg", "DimensionError");
}

#[test]
fn test_sums_need_the_same_dimensions() {
	fails_with("2 m * 3 kg + 1 m", "DimensionError");
	assert_eq!(shown("2 m * 3 kg + 1 m * 1 kg"), "7m·kg");
}
