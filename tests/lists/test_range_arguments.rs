// A range passed to a function that only reads it (count, sum, index, for, map …) stays its bounds (notes/lazy_ranges.md):
// `f(1..10^12)` collects no list, the function gets the range's start and end
use crate::is;
use warp::{int, list};

#[test]
fn a_range_argument_is_read_from_its_bounds() {
	is!("def f(xs) = sum xs; f(1..10)", 45);
	is!("f(xs) := count xs; f(1 to 1000000000000)", 1000000000000i64);
	is!("f(xs) := xs#3; f(5..1000000000)", 7);
	is!("f(k, xs) := k * xs.count; n = 1000000; f(2, 1..n)", 1999998);
	is!("f(xs) := { s = 0; for x in xs { s += x }; s }; f(1..1001)", 500500);
	is!("f(xs) := count(xs.filter(x => x % 3 == 0)); f(1..100001)", 33333);
	is!("f(a, b) := [count a, sum b]; f(1..1000000000000, 1 to 100)", list(vec![int(999999999999), int(5050)]));
}

#[test]
fn a_range_variable_passed_on_stays_its_bounds() {
	is!("f(xs) := xs.count; xs = 1..1000000000000; f(xs)", 999999999999i64);
	is!("def total(xs) = sum xs; n = 10000000; r = 1..n; total(r)", 49999995000000i64);
}

/// a function that changes, returns or passes on its list parameter gets the list, as before
#[test]
fn a_range_argument_used_as_a_list_stays_one() {
	is!("f(xs) := { xs#1 = 9; xs#1 }; f(1..5000)", 9);
	is!("f(xs) := xs; count f(1..5000)", 4999);
	is!("g(ys) := count ys; f(xs) := g(xs); f(1..5)", 4);
	is!("f(xs) := count xs; [f(1..5), f([7, 8])]", list(vec![int(4), int(2)]));
}

/// a range passed on to another function that only reads it stays its bounds all the way (card transitive-range)
#[test]
fn a_range_passed_on_to_a_reader_stays_its_bounds() {
	is!("h(zs) := zs.count; g(ys) := h(ys); f(xs) := g(xs); f(1..1000000000000)", 999999999999i64);
	is!("g(ys) := sum ys; f(k, xs) := k * g(xs); f(2, 1 to 1000000)", 1000001000000i64);
	is!("g(a, b) := a.count; f(xs) := g([1], xs) + g(xs, [1]); f(1..1000000000000)", 1000000000000i64);
	is!("f(xs, n) := { if n == 0 then xs.count else f(xs, n - 1) }; f(1..1000, 3)", 999);
}
