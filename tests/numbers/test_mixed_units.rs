// card units-mixed: a unit written in the program (`1 m`, static) meeting a quantity known only at run time
// (`quantity("5 km")`, lib/units.warp) becomes one too, on either side of the operator
use crate::common::fails_with;
use crate::is;

/// the text of the program's last value, a quantity's own `5km`
fn shown(code: &str) -> String {
	let (statements, last) = code.rsplit_once('\n').unwrap_or(("", code));
	warp::wasm_emitter::eval(&format!("{statements}\nstr({last})")).serialize().trim_matches('"').to_string()
}

#[test]
fn a_static_unit_meeting_a_run_time_quantity_becomes_one() {
	assert_eq!(shown("quantity(\"5 km\") + 1 m"), "5.001km");
	assert_eq!(shown("q = quantity(\"5 km\")\nq - 500 m"), "4.5km");
	assert_eq!(shown("quantity(\"10 m\") / 2 s"), "5m/s");
	assert_eq!(shown("q = quantity(\"3 m\")\nq * 2 m"), "6m²");
	assert_eq!(shown("q = quantity(\"1 km\")\nq + 5 m + 2.5 m"), "1.0075km");
}

#[test]
fn a_static_unit_on_the_left_compares_too() {
	is!("q = quantity(\"5 km\")\n2 km < q", true);
	is!("q = quantity(\"60 s\")\n1 min == q", true);
}

#[test]
fn other_dimensions_still_fail_loudly() {
	fails_with("quantity(\"5 km\") + 5 m/s", "DimensionError: cannot add 5km and 5m/s");
	fails_with("quantity(\"5 km\") + 2 kg", "DimensionError: cannot add 5km and 2kg");
}

#[test]
fn static_units_alone_stay_static() {
	assert_eq!(warp::wasm_emitter::eval("1 km + 1 m").serialize().trim(), "1001m");
}
