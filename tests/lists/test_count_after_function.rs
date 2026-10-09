// card print-people: `print #xs` is print(#xs), the count, not element xs of print: a function is never indexed, nor
// an operand of a subtraction (`print -1`)
use crate::is;

#[test]
fn a_count_after_a_function() {
	is!("f(n) := n * 2; f #[1 2 3]", 6);
	is!("xs = [5 6 7]; xs #2", 6);
}

#[cfg(feature = "native")]
#[test]
fn print_a_count() {
	assert_eq!(crate::common::printed("xs = [1 2 3]; print #xs"), "3\n");
	assert_eq!(crate::common::printed("xs = [1 2 3]; print #(xs where it > 1)"), "2\n");
	assert_eq!(crate::common::printed("print -1"), "-1\n");
}
