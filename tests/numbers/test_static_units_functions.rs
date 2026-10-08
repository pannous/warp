//! Static units, stage 2: a function called with quantities is specialised per unit signature of its arguments
//! (names like s, m, h are units: a program that reuses one as a variable is left to the compile-time evaluator)
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn test_functions_take_and_give_quantities() {
	assert_eq!(shown("speed(d, t) := d/t; v = speed(10 km, 2 h); for i in 1..2 { v = v }; v"), "5 km/h");
	assert_eq!(shown("twice(x) := x * 2; a = twice(3 m); b = twice(4); for i in 1..2 { a = a }; a"), "6 m");
	assert_eq!(shown("area(w, l) := w * l; t = 0 m²; for i in 1..3 { t += area(2 m, 3 m) }; t"), "12 m²");
	assert_eq!(shown("lap() := 400 m; d = 0 m; for i in 1..4 { d += lap() }; d"), "1200 m");
}

#[test]
fn test_one_function_with_several_unit_signatures() {
	assert_eq!(shown("twice(x) := x * 2; a = twice(3 m); b = twice(2 s); for i in 1..2 { a = a }; a / b"), "3/2 m/s");
}

#[test]
fn test_dimension_errors_inside_functions_are_compile_errors() {
	fails_with("total(a, b) := a + b; x = total(1 m, 2 s); for i in 1..2 { x = x }; x", "DimensionError");
}

#[test]
fn test_a_unit_word_called_as_a_function_is_no_unit() {
	// `min(n, 3)` calls min, it is no minute: tk stays a plain function that sp can call (lib/list.wasp split_at)
	assert_eq!(shown("tk(xs, n) := { out = []; for i in 1 to min(n, count(xs)) { out = out + [xs#i] }; out }; sp(xs, n) := [tk(xs, n), 2]; sp([1, 2, 3, 4], 1)"), "[[1] 2]");
}
