//! A for loop over a number is a compile error naming the fix (card non-list; it failed only at run time, "not a list");
//! lists, texts and ranges stay walkable
use crate::common::fails_with;
use crate::is;

#[test]
fn a_for_loop_over_a_number_is_a_compile_error() {
	fails_with("s = 0; for x in 3 { s += x }; s", "walks a number");
	fails_with("n = 3; s = 0; for x in n { s += x }; s", "walks a number");
	fails_with("s = 0.0; for x in 2.5 { s += x }; s", "walks a number");
	is!("s = 0; for x in 1 to 3 { s += x }; s", 6);
	is!("s = 0; for x in [1, 2] { s += x }; s", 3);
	is!("n = 0; for c in \"abc\" { n += 1 }; n", 3);
	is!("n = 3; s = 0; for x in 1 to n { s += x }; s", 6);
}
