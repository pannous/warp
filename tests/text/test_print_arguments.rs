// `print` with several arguments writes them separated by a space, like Python: `print(a, b)` and `print a, b`.
// The call gives nothing (issue #18).
use crate::is;

#[cfg(feature = "native")] // runs the warp binary
use crate::common::printed;

#[test]
#[cfg(feature = "native")]
fn print_joins_its_arguments_with_a_space() {
	assert!(printed("g=\"hi\"; print(g, g)").starts_with("hi hi\n"));
	assert!(printed("g=\"hi\"; print g, g").starts_with("hi hi\n"));
	assert!(printed("print(\"x\", 1, 2.5)").starts_with("x 1 2.5\n"));
	assert!(printed("n=3; print \"n =\", n").starts_with("n = 3\n"));
}

#[test]
#[cfg(feature = "native")]
fn print_of_a_braceless_call_prints_its_result() {
	assert!(printed("print first [10, 5]").starts_with("10\n"));
	assert!(printed("print(upper \"ab\")").starts_with("AB\n"));
	assert!(printed("print upper \"ab\", \"c\"").starts_with("AB c\n"));
}

#[test]
fn print_of_a_braceless_call_gives_nothing() {
	is!("print first [10, 5]", warp::Node::Empty);
	is!("print(upper \"ab\")", warp::Node::Empty);
	is!("x=print upper \"ab\"; x", warp::Node::Empty);
}

#[test]
fn print_of_several_arguments_gives_nothing() {
	is!("g=\"hi\"; print(g, g)", warp::Node::Empty);
	is!("g=\"hi\"; print g, g", warp::Node::Empty);
	is!("n=3; print(\"n\", n)", warp::Node::Empty);
}

/// `print()` writes an empty line, like Python, and gives nothing (issue #18)
#[test]
#[cfg(feature = "native")]
fn print_without_arguments_writes_an_empty_line() {
	assert!(printed("print(); print 1").starts_with("\n1\n"));
	is!("print()", warp::Node::Empty);
	is!("x = print(); x", warp::Node::Empty);
}
