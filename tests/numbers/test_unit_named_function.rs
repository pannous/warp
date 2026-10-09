// card static-units-function: a function or parameter named like a unit shadows only that unit; the other units of the
// program still compute (before, any such name switched the units passes off: "undefined variable: m")
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn a_function_named_like_a_unit_keeps_the_other_units() {
	assert_eq!(shown("g(x) := x + 1 m; g(3 m)"), "4m");
}

#[test]
fn a_parameter_named_like_a_unit_keeps_the_other_units() {
	assert_eq!(shown("f(s) := s * 2; f(3 km)"), "6km");
}

#[test]
fn a_variable_named_like_a_unit_keeps_the_other_units() {
	assert_eq!(shown("g = 9; 3 km + 2 m"), "3002m");
}
