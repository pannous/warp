// P119 (user, 2026-10-06): NaNs are canonicalized where a run shows them, not after every float operation: printed
// any NaN is `NaN`, and a NaN that leaves the run (its result, a task's value, a shared array) is 0x7ff8000000000000
use crate::is;

#[cfg(feature = "native")]
fn shown_bits(code: &str) -> u64 {
	use warp::{Node, Number};
	match warp::wasm_emitter::eval(code).drop_meta() {
		Node::Number(Number::Float(x)) => x.to_bits(),
		Node::Number(Number::Nan) => warp_runtime::floats::CANONICAL_NAN,
		other => panic!("{code} gave {other:?}"),
	}
}

#[test]
fn a_nan_prints_as_nan() {
	is!("y = -0.0/0.0; \"a\" + y", "aNaN");
	is!("y = (0.0/0.0) * -1.0; \"\\(y)\"", "NaN");
}

#[test]
#[cfg(feature = "native")]
fn a_nan_leaving_the_run_is_canonical() {
	const CANONICAL_NAN: u64 = 0x7ff8_0000_0000_0000;
	assert_eq!(shown_bits("x = -0.0 as float; x / 0.0"), CANONICAL_NAN);
	assert_eq!(shown_bits("f(x:float) := -x / 0.0; await go f(0.0)"), CANONICAL_NAN);
	assert_eq!(shown_bits("shared xs = float[1]; xs#1 = -0.0 / 0.0; xs#1 * -1.0"), CANONICAL_NAN);
}
