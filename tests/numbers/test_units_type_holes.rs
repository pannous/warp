// The unit holes the W0 type model found (tests/types/test_type_model.rs): cards units-compare, units-reassign,
// units-annotation, units-text-repeat
use crate::common::fails_with;
use crate::is;

#[test]
fn a_comparison_of_quantities_is_yes_or_no() {
	is!("1 m < 2 m", true);
	is!("50 cm < 1 m", true);
	is!("1 m == 100 cm", true);
	is!("2 km < 3 m", false);
	is!("x = 3 m; y = x * 2; y < 7 m", true);
}

#[test]
fn a_variable_keeps_its_dimension() {
	fails_with("x = 1 m; x = 2 s; x", "DimensionError");
	is!("x = 1 m; x = 2 km; x == 2000 m", true);
}

#[test]
fn a_number_annotation_takes_no_quantity() {
	fails_with("x: int = 1 m; x", "DimensionError");
}

#[test]
fn a_text_repeats_a_plain_count_of_times() {
	fails_with("2 m * \"a\"", "DimensionError");
	fails_with("\"ab\" * 2 m", "DimensionError");
	is!("\"a\" * 2", "aa");
}
