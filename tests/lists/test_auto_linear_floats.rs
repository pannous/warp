// card auto-linear-floats: a plain `xs = float[n]` mapped by a numeric kernel lives in linear memory by itself, so the
// map runs as the f64x2 kernel or one loop into a new block (notes/simd.md), when every use is one linear arrays support
use crate::is;
use warp::{float, int, list};

const FILLED: &str = "xs = float[5]; for i in 1 to 5 { xs#i = i * i * 1.0 }\n";

#[test]
fn a_float_array_mapped_by_a_kernel_is_linear() {
	is!(
		&format!("{FILLED}ys = xs.map(x => x * 0.5 + 1); ys"),
		list(vec![float(1.5), float(3.0), float(5.5), float(9.0), float(13.5)])
	);
	is!(&format!("{FILLED}ys = map(xs, v => -sqrt(v) / 2); [ys#5, #ys]"), list(vec![float(-2.5), int(5)]));
	let numeric = format!("{FILLED}ys = xs.map(x => max(x, 2) * 2); [ys#1, ys#3, #ys]");
	is!(&numeric, list(vec![float(4.0), float(18.0), int(5)]));
	let captured = format!("{FILLED}k = 3.0; ys = xs.map(x => x * k); ys#2");
	is!(&captured, 12.0);
}

#[test]
fn a_float_array_mapped_otherwise_stays_a_list() {
	let called = format!("{FILLED}triple(v) := v * 3.0; ys = xs.map(x => triple(x)); ys#2");
	is!(&called, 12.0);
	let printed = format!("{FILLED}print(xs); ys = xs.map(x => x * 2.0); ys#2");
	is!(&printed, 8.0);
}

#[test]
fn a_float_kernel_may_group_and_call_sqrt() {
	let grouped = format!("{FILLED}ys = xs.map(x => sqrt(x) * (x + 1)); [ys#1, ys#3]");
	is!(&grouped, list(vec![float(2.0), float(30.0)]));
}
