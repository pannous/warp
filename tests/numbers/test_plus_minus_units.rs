// card plus-minus-units: a written tolerance with units the compile-time evaluator can't answer becomes a run-time
// Quantity whose amount is a ± value (units::lower_run_time_tolerances, notes/plus_minus.md)
use crate::common::fails_with;
use crate::is;

fn shown(code: &str) -> String {
	warp::wasm_emitter::eval(code).serialize().trim_matches('"').to_string()
}

#[test]
fn a_tolerance_with_units_flows_through_arithmetic() {
	assert_eq!(shown("x = 5 m ± 1 cm; str(x * 2)"), "10.000 ± 0.020m");
	assert_eq!(shown("area = (5 m ± 1 cm) * (2 m ± 1 cm); str(area)"), "10.000 ± 0.070m²");
	assert_eq!(shown("str((5 m ± 1 cm) + 1 m)"), "6.0000 ± 0.0100m");
	assert_eq!(shown("x = 2 km/h ± 1 m/s; str(x * 2)"), "4.0 ± 7.2km/h");
}

#[test]
fn a_plain_spread_counts_in_the_value_unit() {
	assert_eq!(shown("x = 5 m ± 1; str(x + 1 m)"), "6.0 ± 1.0m");
}

#[test]
fn a_tolerance_with_units_passes_through_functions() {
	assert_eq!(shown("f(x) := x * 2; str(f(5 m ± 1 cm))"), "10.000 ± 0.020m");
	assert_eq!(shown("f(x:Quantity) := x * 2; str(f(quantity(\"5 m\")))"), "10m");
	assert_eq!(shown("str((quantity(\"5 m\")) * 2)"), "10m");
}

#[test]
fn compile_time_tolerances_stay_at_compile_time() {
	is!("1900 - 2000 AD == 1950 AD ± 50", 1);
	is!("(5 ± 1) is number", true);
}

#[test]
fn a_spread_of_another_dimension_fails() {
	fails_with("x = 5 m ± 1 s; str(x * 2)", "Dimension");
}
