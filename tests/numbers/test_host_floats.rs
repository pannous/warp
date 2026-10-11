// decision exact-decimals: host readings (the pointer) are float sources, and an exact operation a float uses
// (`col + 0.5 - x` of a float x) takes the f64 operation when its operands are f64s exactly, the value it would
// convert to anyway (card exact-decimal, probes/perf/finger_paint_bench.warp: a stroke 5 → 0.3 ms)
use crate::is;

#[test]
fn the_pointer_is_a_float_source() {
	is!("type(mouse_x)", "float");
	is!("type(mouse_y + 0.5)", "float");
}

#[test]
fn an_exact_operation_a_float_uses_rounds_once() {
	is!("col = 3; col + 0.5 - 0.25f", 3.25);
	is!("col = 3; 0.5 * col - 0.25f", 1.25);
	// beyond 2^53, a ratio, a decimal no f64 holds: the exact operation, then its f64
	is!("big = 2^53 + 1; big + 0.5 - 0.0f", 9007199254740994.0);
	is!("third = 1/3; third + 0.5 - 0.0f", 0.8333333333333334);
	is!("col = 3; col + 0.1 - 0.0f", 3.1);
}
