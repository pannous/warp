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

#[test]
fn dimension_names_come_from_the_definitions() {
	is!("type(2 m²)", "area");
	is!("type(5 Hz)", "frequency");
	is!("type(2 AD)", "era");
}

// decision unit-definitions: lib/units.warp's lines, `dimension length meter` an anchor and `unit m := 1 meter` its
// symbol, derived units of their expression, SI prefixes by themselves
#[test]
fn units_come_from_unit_lines() {
	is!("str(3 meter)", "3m");
	is!("1 µs == 1000 ns", true);
	is!("1 Mg == 1000 kg", true);
	is!("2 L == 2000 cm³", true);
	is!("1 N == 1 kg*m/s²", true);
}

// a program adds a unit by a unit line, an amount of a known unit; a plain `:=` stays a constant (decision
// unit-definitions)
#[test]
fn a_program_adds_units_by_unit_lines() {
	is!("unit furlong := 201.168 m\nx = 1 km\nstr(x as furlong)", "(62500/12573)furlong");
	is!("unit furlong := 201.168 m\n3 furlong == 603.504 m", true);
	is!("unit fortnight := 14 * 24 h\nstr(336 h as fortnight)", "1fortnight");
	is!("unit furlong := 201.168 m\nquantity(1, \"furlong\").in(\"m\")", 201.168);
	is!("distance := 42 km\nstr(distance)", "42km");
	is!("unit furlong := 201.168m; str(1km as furlong)", "(62500/12573)furlong");
}

// decision unit-target: `as` names the unit the right side counts in, `it` the amount in the new unit
#[test]
fn a_unit_line_names_its_target_by_as() {
	is!("212°F == 100°C", true);
	is!("unit rankine as kelvin := it * 5/9\n9 rankine == 5 K", true);
	is!("unit hand as inch := 4 * it\nstr(1 hand as cm)", "10.16cm");
}
