use warp::wasm_emitter::eval;
use crate::is;

fn text_of(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn quantities_compare_in_the_finer_unit() {
	is!("3km == 3000m", true);
	is!("3km == 3001m", false);
	is!("3km != 3001m", true);
	is!("3km > 2999m", true);
	is!("3km < 2999m", false);
	is!("3km >= 3000m", true);
	is!("2999m <= 3km", true);
}

#[test]
fn scalar_division_keeps_the_unit_and_same_dimension_division_is_a_number() {
	assert_eq!(text_of("6 m / 2"), "3m");
	is!("6 m / 2 m", 3);
	is!("6 km / 2000 m", 3);
}

#[test]
fn division_by_another_dimension_is_a_composite_unit() {
	assert_eq!(text_of("10 km / 2 h"), "5km/h");
	assert_eq!(text_of("10 km / 2h"), "5km/h");
}

#[test]
fn mixed_dimensions_are_a_loud_dimension_error() {
	for code in ["1 km + 1 s", "1 km - 1 s", "1 km == 1 s", "1 km < 1 s"] {
		let text = text_of(code);
		assert!(text.contains("DimensionError"), "{code}: {text}");
	}
}

#[test]
fn a_variable_may_change_from_nothing_to_a_number_in_a_loop() {
	is!("x=ø; while x missing {x=1}; x", 1);
	is!("x=ø; while x == ø {x=1}; x", 1);
	is!("x=ø; while (not x) {x=2}; x", 2);
}
