// An accumulator a function starts at `out = 0` widens to the floats of unknown origin it adds up, as a main-level
// one does (card float-accumulator)
use crate::is;

#[test]
fn a_function_accumulator_takes_floats_of_unknown_origin() {
	is!("s(xs) := { out = 0; for x in xs { out = out + x }; out }; s([float(1.5), float(2)])", 3.5);
	is!("s(xs) := { out = 0; for x in xs { out += x }; out }; s([float(1.5), float(2)])", 3.5);
	is!("s(xs) := { out = 0; for x in xs { out = out + x }; out }; s([1, 2])", 3);
	is!("s(xs) := { y = 0.5; for x in xs { y = x }; y }; s([float(1.5)])", 1.5);
}

#[test]
fn sum_by_takes_a_float_lambda() {
	is!("use list; sum_by([float(1), float(2)], x => x * 1.5)", 4.5);
}

/// a Node local a nested function captures is set before the calls in its own first value refresh the captures
#[test]
fn a_captured_node_local_is_set_before_its_first_value() {
	assert_eq!(warp::wasm_emitter::eval("g(xs) := xs#1; v(xs) := { m = g(xs); f = x => [x, m]; f(1) }; v([\"b\", 4])").serialize(), "[1 'b']");
}
