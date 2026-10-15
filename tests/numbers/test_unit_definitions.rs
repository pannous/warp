// card dimensions-units Q2: the dimensions and units come from lib/units.warp's unit_definitions block
// (src/numbers/units/definitions.rs), each unit an exact amount of another; temperatures' zeros: test_temperature.rs
use crate::is;

#[test]
fn each_unit_is_the_amount_its_definition_says() {
	is!("1 km == 1000 m", true);
	is!("1 inch == 2.54 cm", true);
	is!("1 mi == 1609.344 m", true);
	is!("1 lb == 0.45359237 kg", true);
	is!("1000 mg == 1 g", true);
	is!("1 h == 3600 s", true);
	is!("1 mph == 1 mi/h", true);
	is!("1 kHz == 1000 Hz", true);
}

#[test]
fn dimension_names_come_from_the_definitions() {
	is!("type(2 m²)", "area");
	is!("type(5 Hz)", "frequency");
	is!("type(2 AD)", "era");
}
