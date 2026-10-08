// `quantity in unit` and `quantity as unit` (user, P36): the quantity in another unit of its dimension, exact when it
// divides, else a ratio; long unit names work as targets; a unit of another dimension is the DimensionError
use warp::wasm_emitter::eval;

fn text_of(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_quantity_converts_to_a_unit_of_its_dimension() {
	assert_eq!(text_of("100 cm in m"), "1m");
	assert_eq!(text_of("3 km as m"), "3000m");
	assert_eq!(text_of("2 h in min"), "120min");
	assert_eq!(text_of("1500 g in kg"), "1.5kg");
}

#[test]
fn an_inexact_conversion_is_a_ratio() {
	assert_eq!(text_of("150 cm in m"), "1.5m");
	assert_eq!(text_of("1 m in km"), "0.001km");
}

#[test]
fn long_unit_names_name_the_target() {
	assert_eq!(text_of("2 h in minutes"), "120min");
	assert_eq!(text_of("90 min as hours"), "1.5h");
	assert_eq!(text_of("3 km in meters"), "3000m");
}

#[test]
fn another_dimension_is_the_dimension_error() {
	assert!(text_of("1 km in kg").contains("DimensionError"));
}

#[test]
fn a_duration_converts_to_a_time_unit() {
	assert_eq!(text_of("2 hours in minutes"), "120min");
	assert_eq!(text_of("90 minutes as hours"), "1.5h");
	assert_eq!(text_of("1 day in h"), "24h");
	assert!(text_of("2 hours in m").contains("DimensionError"));
}
