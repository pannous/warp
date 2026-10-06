//! `for i in 0..n { … }`: a variable range bound followed by a block is a loop body, not a call or definition of n

use crate::is;

#[test]
fn a_variable_range_end_keeps_the_block_as_loop_body() {
	is!("n=4; s=0; for i in 0..n { s += i }; s", 6);
	is!("n=4; s=0; for i in 0..<n { s += i }; s", 6);
	is!("n=4; s=0; for i in (0..n) { s += i }; s", 6);
	is!("n=4; s=0; for i in 0 until n { s += i }; s", 6);
	is!("n=3; s=0; for i in 0...n { s += i }; s", 6);
	is!("n=3; s=0; for i in 0 to n { s += i }; s", 6);
	is!("n=4; s=0; for i in 0 upto n { s += i }; s", 6);
	is!("a=1; b=4; s=0; for i in a..b { s += i }; s", 6);
	is!("count(n) := { c=0; for i in 0..n { c += 1 }; c }; count(5)", 5);
	is!("xs=[1,2,3]; s=0; for x in xs { s += x }; s", 6);
	is!("n=4; s=0; for i in 0..n do s += i; s", 6);
	is!("n=4; s=0; for i in 0..n do { s += i }; s", 6);
}

#[test]
fn range_calls_are_exclusive_ranges() {
	is!("n=4; s=0; for i in range(n) { s += i }; s", 6);
	is!("s=0; for i in range(1, 4) { s += i }; s", 6);
	is!("s=0; for i in range(4) { s += i }; s", 6);
}
