// `linear xs = int[n]` / `float[n]`: an explicit array in linear memory, one block [count][cells] (user 2026-10-06);
// discouraged with a hint, since the compiler picks where number lists live by itself (notes/linear_arrays.md)
use crate::common::fails_with;
use crate::is;
use warp::{float, int, ints, list};

#[test]
fn a_linear_array_reads_and_writes_its_cells() {
	is!("linear xs = int[5]; xs#2 = 7; xs#3 += 4; [xs#2, xs#3, #xs, xs#1, xs.count]", ints(vec![7, 4, 5, 0, 5]));
	is!("linear ys = float[3]; ys#1 = 1.5; ys#1 += 0.25; ys#1", 1.75);
	is!("linear xs = int[1000000]; for i in 1 to 1000000 { xs#i = i }; s = 0; for i in 1 to 1000000 { s += xs#i }; s", 500000500000i64);
}

#[test]
fn a_linear_array_is_a_list_as_a_whole() {
	is!("linear xs = int[3]; xs#1 = 4; xs", ints(vec![4, 0, 0]));
	is!("linear ys = float[2]; ys#2 = 2.5; s = 0.0; for y in ys { s += y; s += 1.0 }; [s, ys]", list(vec![float(4.5), list(vec![float(0.0), float(2.5)])]));
	is!("linear xs = int[4]; f(a) := a#2 + 1; xs#2 = 5; f(xs)", 6);
	is!("linear xs = int[2]; xs#2 = 3; t = 0; for x in xs { t += x }; t", int(3));
}

#[test]
fn a_linear_array_checks_its_bounds_and_stays_in_its_task() {
	fails_with("linear xs = int[3]; xs#4", "index out of range");
	fails_with("linear xs = int[3]; xs#0 = 1", "index out of range");
	fails_with("linear xs = int[4]; g(a) := a#1; await go g(xs)", "a task cannot take the linear array xs");
}
