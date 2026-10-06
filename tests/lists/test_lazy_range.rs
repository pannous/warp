// A range is never expanded item by item at compile time (card lazy-range): `xs = (1..100001)` overflowed the
// compiler's stack building a literal list of every number
use crate::is;
use warp::{int, ints, list};

#[test]
fn a_big_range_variable_compiles() {
	is!("xs=(1..100001); count xs", 100000);
	is!("xs = 1..100001; xs#99999", 99999);
	is!("xs = 1 to 100000; sum xs", 5000050000i64);
	is!("xs = 1..100001; count(xs.filter(x => x % 2 == 0))", 50000);
	is!("xs = 1..100001; (xs.map(x => x * 2))#100000", 200000);
}

/// count, index and sum read the bounds: a trillion numbers are never collected
#[test]
fn a_range_is_read_from_its_bounds() {
	is!("xs = 1 to 1000000000000; [#xs, xs#999999999999]", list(vec![int(1000000000000), int(999999999999)]));
	is!("xs = 1..1000000000000; xs.count", 999999999999i64);
	is!("sum(1 to 10000000000) == 50000000005000000000", true);
	is!("n = 5000; xs = 3..n; for x in xs { y = x }; [y, xs.sum]", ints(vec![4999, 12497497]));
	is!("count(5..1)", 0);
	is!("f(n) := { xs = 1..n; count xs }; f(100000)", 99999);
}

/// a changed, captured or passed-on range variable is a list as before
#[test]
fn a_range_variable_used_as_a_list_stays_one() {
	is!("xs = 1..5000; xs#2 = 9; xs#2", 9);
	is!("xs = 1..5000; ys = xs; count ys", 4999);
	is!("xs = 1..5000; f(x) := xs#1; f(0)", 1);
	crate::common::fails_with("xs = 1..100001; xs#0", "index out of range");
}

#[test]
fn a_small_range_variable_is_still_its_list() {
	is!("xs = 1..5; xs", ints(vec![1, 2, 3, 4]));
	is!("xs = 1..5; xs#2 = 7; xs", ints(vec![1, 7, 3, 4]));
}

#[test]
fn a_range_bound_of_arithmetic_stays_a_descriptor() {
	// card lazy-range-power: `1..10^12` was collected (out of fuel) while `1..n` with n = 10^12 was not
	is!("count 1..10^12", 999999999999i64);
	is!("r = 1..10^12; r#7", 7);
	is!("n = 5; r = 1..n+2; count r", 6);
}
