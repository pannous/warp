//! A range from a letter to a number is a compile error naming both readings (card range-mixed; it failed only at run
//! time with "not a character"); letter and number ranges stay as they are
use crate::common::fails_with;
use crate::is;

#[test]
fn a_range_from_a_letter_to_a_number_is_a_compile_error() {
	fails_with("\"a\"..3", "mixes a letter and a number");
	fails_with("3 to 'c'", "mixes a letter and a number");
	fails_with("for c in \"a\" to 3 { print c }", "mixes a letter and a number");
	is!("s = 0; for x in 1..3 { s += x }; s", 3);
	is!("n = 0; for c in 'a' to 'c' { n += 1 }; n", 3);
}
