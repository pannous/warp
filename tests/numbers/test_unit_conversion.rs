// `quantity in unit` and `quantity as unit` (user, P36): the quantity in another unit of its dimension, exact when it
// divides, else a ratio; long unit names work as targets; a unit of another dimension is the DimensionError
use warp::wasm_emitter::eval;

fn text_of(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_quantity_converts_to_a_unit_of_its_dimension() {
	assert_eq!(text_of("100 cm in m"), "1 m");
	assert_eq!(text_of("3 km as m"), "3000 m");
	assert_eq!(text_of("2 h in min"), "120 min");
	assert_eq!(text_of("1500 g in kg"), "3/2 kg");
}

#[test]
fn an_inexact_conversion_is_a_ratio() {
	assert_eq!(text_of("150 cm in m"), "3/2 m");
	assert_eq!(text_of("1 m in km"), "1/1000 km");
}

#[test]
fn long_unit_names_name_the_target() {
	assert_eq!(text_of("2 h in minutes"), "120 min");
	assert_eq!(text_of("90 min as hours"), "3/2 h");
	assert_eq!(text_of("3 km in meters"), "3000 m");
}

#[test]
fn another_dimension_is_the_dimension_error() {
	assert!(text_of("1 km in kg").contains("DimensionError"));
}

#[test]
fn a_duration_converts_to_a_time_unit() {
	assert_eq!(text_of("2 hours in minutes"), "120 min");
	assert_eq!(text_of("90 minutes as hours"), "3/2 h");
	assert_eq!(text_of("1 day in h"), "24 h");
	assert!(text_of("2 hours in m").contains("DimensionError"));
}
