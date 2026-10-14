//! A loop ending in an `if` gives the if's value of its last pass (P55): ø when that pass took no branch (card loop-if);
//! a loop whose value is dropped computes none, so filtering a long list stays linear
use crate::is;
use warp::*;

#[test]
fn a_loop_ending_in_an_if_gives_the_last_pass_value() {
	is!("for x in [1 2 3] { if x > 2 then { \"big\" } }", "big");
	is!("for x in [1 2 3] { if x < 2 then { \"small\" } }", Empty);
	is!("for x in [1 2] { if x > 1 { \"y\" } else { \"n\" } }", 'y');
	is!("out = []; for x in [1 2 3] { if x > 1 then { out = out + [x] } }", ints(vec![2, 3]));
}

#[test]
fn a_dropped_loop_value_keeps_filtering_linear() {
	is!("xs = 1..100001; count(xs.filter(x => x % 2 == 0))", 50000);
}
