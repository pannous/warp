//! `sum` of a list of decimal literals is exact

use warp::is;

#[test]
fn the_sum_of_decimal_elements() {
	is!("sum [1.5 2.5]", 4);
	is!("sum([1.5 2.5])", 4);
	is!("xs=[1.5 2.5]; sum(xs)", 4);
	is!("xs=[0.1 0.2]; sum(xs) == 0.3", true);
}
