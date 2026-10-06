// A range is never expanded item by item at compile time (card lazy-range): `xs = (1..100001)` overflowed the
// compiler's stack building a literal list of every number
use crate::is;
use warp::ints;

#[test]
fn a_big_range_variable_compiles() {
	is!("xs=(1..100001); count xs", 100000);
	is!("xs = 1..100001; xs#99999", 99999);
	is!("xs = 1 to 100000; sum xs", 5000050000i64);
	is!("xs = 1..100001; count(xs.filter(x => x % 2 == 0))", 50000);
	is!("xs = 1..100001; (xs.map(x => x * 2))#100000", 200000);
}

#[test]
fn a_small_range_variable_is_still_its_list() {
	is!("xs = 1..5; xs", ints(vec![1, 2, 3, 4]));
	is!("xs = 1..5; xs#2 = 7; xs", ints(vec![1, 7, 3, 4]));
}
