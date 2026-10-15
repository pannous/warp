// print-bang (user 2026-10-15, wiki/partial-evaluation.md): a function that gives nothing never takes D2's in-place
// bang: `print friends!` prints the forced friends and friends stays; `sort xs!` still sorts in place
use crate::is;

#[test]
fn print_of_a_forced_value_keeps_the_variable() {
	is!("friends = [1 2]; print friends!; count friends", 2);
	is!("friends = \"ab\"; puts friends!; friends", "ab");
	is!("xs = [3 1 2]; sort xs!; xs#1", 1);
}

#[cfg(feature = "native")]
#[test]
fn print_of_a_forced_value_prints_it() {
	assert_eq!(crate::common::printed("friends = [1 2]; print friends!").trim(), "[1 2]");
}
