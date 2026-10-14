// card units-dynamic: quantities whose unit is known only at run time (lib/units.warp, notes/units_runtime.md "Stage 8"):
// `quantity("5 km")` read from text, `quantity(5, unit)` of a unit given as data, carrying their unit as data
use crate::common::fails_with;
use crate::is;

/// the text of the program's last value, a quantity's own `5km`
fn shown(code: &str) -> String {
	let (statements, last) = code.rsplit_once('\n').unwrap_or(("", code));
	warp::wasm_emitter::eval(&format!("{statements}\nstr({last})")).serialize().trim_matches('"').to_string()
}

#[test]
fn a_quantity_read_from_text_carries_its_unit() {
	assert_eq!(shown("quantity(\"5 km\")"), "5km");
	assert_eq!(shown("u = \"km\"\nquantity(5, u)"), "5km");
	assert_eq!(shown("quantity(\" -3.5kg \")"), "-3.5kg");
	is!("quantity(\"2.5 km\").in(\"m\")", 2500);
	is!("quantity(\".5 h\").in(\"min\")", 30);
}

#[test]
fn quantities_of_one_dimension_add_and_compare() {
	assert_eq!(shown("quantity(\"5 km\") + quantity(\"250m\")"), "5.25km");
	assert_eq!(shown("quantity(\"1 km\") - quantity(\"1 m\")"), "0.999km");
	is!("quantity(\"1 km\") > quantity(\"999 m\")", true);
	is!("quantity(\"1 min\") == quantity(\"60 s\")", true);
	assert_eq!(shown("d = quantity(\"3 m\")\ne = d * 2\ne + d"), "9m");
}

#[test]
fn quantities_multiply_divide_and_convert() {
	assert_eq!(shown("quantity(\"3 m\") * quantity(\"2 m\")"), "6m²");
	assert_eq!(shown("quantity(\"10 m\") / quantity(\"2 s\")"), "5m/s");
	assert_eq!(shown("quantity(\"72 km/h\").to(\"m/s\")"), "20m/s");
	is!("quantity(\"6 m²\").in(\"cm²\")", 60000);
}

#[test]
fn quantities_of_other_dimensions_fail_loudly() {
	fails_with("quantity(\"3 m\") + quantity(\"2 s\")", "DimensionError: cannot add 3m and 2s");
	fails_with("quantity(\"3 m\") + 2", "DimensionError: cannot add 3m and the plain number 2");
	fails_with("quantity(\"3 m\").to(\"kg\")", "DimensionError: cannot convert 3m and 1kg");
	fails_with("quantity(\"3 furlong\")", "unknown unit: furlong");
	fails_with("quantity(\"km\")", "not a quantity: km");
}

#[test]
fn use_units_says_it_too() {
	assert_eq!(shown("use units\nquantity(\"5 km\") + quantity(\"1 m\")"), "5.001km");
}

#[test]
fn only_a_program_calling_quantity_carries_the_units_module() {
	let bytes = |program: &str| warp::wasm_emitter::compile(program).unwrap().bytes;
	let contains = |module: &[u8], word: &str| module.windows(word.len()).any(|window| window == word.as_bytes());
	assert!(!contains(&bytes("print \"hello\""), "unit_measure"));
	assert!(contains(&bytes("print quantity(\"5 km\")"), "unit_measure"));
	// a program's own quantity wins
	is!("quantity(x) := x * 2\nquantity(4)", 8);
}
