//! In `if not x {…}` and `while not x {…}` the block is the body, not an argument of x

use warp::is;

#[test]
fn a_negated_condition_takes_its_block() {
	is!("x=2; if not x {1} else {3}", 3);
	is!("x=0; if not x {1} else {3}", 1);
	is!("x=2; if !x {1} else {3}", 3);
}

#[test]
fn a_negated_condition_in_a_loop() {
	is!("x=0; n=0; while not x {n=n+1; x=1}; n", 1);
}
