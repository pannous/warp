// A literal range used as a value is the list of its numbers wherever it stands, as `x = 1..5` is: printed, converted,
// typed, parenthesized, passed on (wiki range.md)
use warp::ints;
use crate::is;

#[test]
fn a_range_is_a_list_as_an_argument() {
	// an argument of the program's own function: print gives nothing (issue #18)
	is!("f(x) := x; f(1..5)", ints(vec![1, 2, 3, 4]));
	is!("f(x) := x; f(1…3)", ints(vec![1, 2, 3]));
	is!("str(1..5)", "[1 2 3 4]");
	is!("type(1..5)", "list of int");
	is!("count(1..5)", 4);
}

#[test]
fn a_parenthesized_range_is_a_list() {
	is!("y = (1..5); y", ints(vec![1, 2, 3, 4]));
	is!("y = (1..5); f(x) := x; f(y)", ints(vec![1, 2, 3, 4])); // print gives nothing (issue #18)
	is!("y = (1..5); y#2", 2);
}

#[test]
fn a_range_of_computed_bounds_is_a_list_as_an_argument() {
	is!("a=2; b=5; f(v) := v; f(a..b)", ints(vec![2, 3, 4])); // print gives nothing (issue #18)
	is!("a=2; b=5; str(a..b)", "[2 3 4]");
}

/// `puts` writes the text of any value, without a line break: a range, a list, a number variable (it wrote nothing, or
/// the variable's name, and gave 0)
#[cfg(feature = "native")]
#[test]
fn puts_writes_the_text_of_a_value() {
	assert!(crate::common::printed("puts 1..5").starts_with("[1 2 3 4]"));
	assert!(crate::common::printed("puts [1, 2]").starts_with("[1 2]"));
	assert!(crate::common::printed("x = 3; puts x").starts_with('3'));
}
