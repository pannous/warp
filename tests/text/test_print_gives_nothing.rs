// `print x` writes x and gives nothing, ø (user, issue #18: "print should not return a value")
use crate::is;
use warp::Node;

#[test]
fn print_gives_nothing() {
	is!("print 3", Node::Empty);
	is!("x = 3; print x", Node::Empty);
	is!("print \"a\", 2", Node::Empty);
	is!("print [1, 2]", Node::Empty);
	is!("print()", Node::Empty);
	is!("f(x) := { print(x); x }; f(3)", 3);
	is!("xs = [1 2]; xs.each(x => print x); 7", 7);
}

/// what print writes is unchanged; the console shows no value after it
#[test]
#[cfg(feature = "native")]
fn the_console_shows_no_value_after_a_print() {
	assert_eq!(crate::common::printed("print 3"), "3\n");
	assert_eq!(crate::common::printed("x = 2; print x; x + 1"), "2\n» 3\n");
}
