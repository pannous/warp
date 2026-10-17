// A block or mutating method call left of and/or runs before the right side (it was dropped as if a literal)
use crate::is;

#[test]
fn a_block_left_of_and_runs() {
	is!("x = 1; (x = 5; true) and x == 5", true);
	is!("class K { n = 0; bump() := { n += 1; true } }; k = K(); k.bump() and k.n == 1", true);
	is!("[] or 3", 3);
	is!("\"a\" and 4", 4);
}
