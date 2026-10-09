//! card print-error: `print ø` prints ø, as `x = ø; print x` does; ø was typed a number and failed to join
use crate::is;

#[test]
fn print_empty_gives_nothing() {
	is!("print ø", warp::Node::Empty);
	is!("print nil", warp::Node::Empty);
	is!("print(ø)", warp::Node::Empty);
	is!("print ø; 3", 3);
}

#[cfg(all(unix, feature = "native"))]
#[test]
fn print_empty_prints_it() {
	let printed = crate::common::printed("print ø\nprint(nil)");
	assert_eq!(printed.lines().filter(|line| !line.starts_with('»')).collect::<Vec<_>>(), ["ø", "ø"]);
}
