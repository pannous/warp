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

// card lib-units: the block is also the table of run-time quantities, aliases of units (Hz) included
#[test]
fn run_time_quantities_know_the_units_of_the_definitions() {
	is!("quantity(1, \"mi\").in(\"m\") == 1609.344", true);
	is!("quantity(440, \"Hz\").in(\"kHz\") == 0.44", true);
}

// a program's `furlong := 201.168 m` stays its constant and is also a unit to convert to (decision dimensions-units,
// rule C after the reviewer's veto: `distance := 42 km` keeps printing 42km)
#[test]
fn a_program_defines_its_own_units() {
	is!("furlong := 201.168 m\nx = 1 km\nstr(x as furlong)", "(62500/12573)furlong");
	is!("furlong := 201.168 m\n3furlong == 603.504 m", true);
	is!("fortnight := 14 * 24 h\nx = 336 h\nstr(x as fortnight)", "1fortnight");
	is!("distance := 42 km\nstr(distance)", "42km");
	is!("n := 5\nn * 2", 10);
}

#[test]
fn dimension_names_come_from_the_definitions() {
	is!("type(2 m²)", "area");
	is!("type(5 Hz)", "frequency");
	is!("type(2 AD)", "era");
}
