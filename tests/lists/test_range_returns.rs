// Card range-value-run-2: a function whose value is a range of its parameters returns the range, not its list: a
// call with plain arguments is that range, so `count f(10^12)` collects nothing (notes/lazy_ranges.md)
use crate::is;
use warp::{int, list};

#[test]
fn a_returned_range_is_read_from_its_bounds() {
	is!("f(n) := 1..n; count f(1000000000000)", 999999999999i64);
	is!("f(n) := 1..n; sum f(10^7)", 49999995000000i64);
	is!("f(n) := 1..n; f(10^12)#5", 5);
	is!("span(a, b) := a to b; count span(3, 10^12)", 999999999998i64);
	is!("evens(n) := 0..2*n; count evens(10^12)", 2000000000000i64);
}

#[test]
fn a_returned_range_stored_in_a_variable_stays_its_bounds() {
	is!("f(n) := 1..n; r = f(10^12); count r", 999999999999i64);
	is!("f(n) := 1..n; total(xs) := sum xs; total(f(10^7))", 49999995000000i64);
}

#[test]
fn a_returned_range_used_as_a_list_is_its_list() {
	is!("f(n) := 1..n; f(5)", list(vec![int(1), int(2), int(3), int(4)]));
	is!("f(n) := 1..n; m = 20; count f(m)", 19);
	is!("f(n) := 1..n; xs = f(4); xs#1 = 9; xs", list(vec![int(9), int(2), int(3)]));
	is!("three() := 3; f(n) := 1..n; count f(three())", 2);
	// a call in the argument runs once
	is!("n = 0; next() := { global n; n += 1; n }; f(k) := 1..k; count f(next() + 4)", 4);
}
