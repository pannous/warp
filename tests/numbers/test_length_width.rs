// card length-width: `6.0m ± 0.3` is a measured length whose plain tolerance counts in the value's unit (6.0m ± 0.3m);
// derived quantities carry the spread (samples/numbers/measurements.warp)
use crate::is;

fn shown(code: &str) -> String {
	warp::wasm_emitter::eval(code).serialize().trim_matches('"').to_string()
}

const ROOM: &str = "length = 6.0m ± 0.3; width = 2.0m ± 0.2; area = length * width; ";

#[test]
fn a_decimal_length_with_a_plain_tolerance_multiplies_into_an_area() {
	assert_eq!(shown(&format!("{ROOM}str(area)")), "12.0 ± 1.9m²");
	assert_eq!(shown(&format!("{ROOM}str(area.low)")), "10.26m²");
	assert_eq!(shown(&format!("{ROOM}str(area.value)")), "12m²");
	assert_eq!(shown("length = 6.0m ± 0.3; width = 2.0m ± 0.2; str((length + width) * 2)"), "16.0 ± 1.0m");
}

#[test]
fn an_area_with_tolerance_compares_with_an_area() {
	is!("length = 6.0m ± 0.3; width = 2.0m ± 0.2; area = length * width; area certainly > 10m²", true);
	is!("length = 6.0m ± 0.3; width = 2.0m ± 0.2; area = length * width; area possibly > 14m²", false);
}

#[test]
#[ignore = "length-width: a function called with a ± quantity and a plain number makes area.low fail: not a Quantity"]
fn bounds_survive_a_function_called_with_a_quantity_and_a_number() {
	assert_eq!(shown(&format!("{ROOM}twice(x) := x * 2; twice(length); twice(3); str(area.low)")), "10.26m²");
}
