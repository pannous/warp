// `print "x changed to " value` (wiki/signal.md): a text followed by values prints them joined without separator
use warp::is;

#[test]
fn a_text_next_to_values_prints_joined() {
	is!("x=3; print \"a \" x", "a 3");
	is!("x=10; on set x {print \"x changed to \" value}; x=3", "x changed to 3");
}

#[test]
#[cfg(feature = "native")]
fn the_joined_text_is_written() {
	assert!(crate::common::printed("x=3; print \"x is \" x \"!\"").starts_with("x is 3!\n"));
}
