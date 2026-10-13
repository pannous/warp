//! `on set x {print "x changed to " value}` with x written inside an expression, `while x-->0`: the write gave the
//! handler's block as its value, "not an int"; it gives x (card on-set-value, wiki/signal.md)
#[cfg(feature = "native")]
use crate::common::printed;
use crate::is;

#[test]
fn a_write_in_an_expression_runs_the_handler_and_gives_the_value() {
	is!("x = 3; on set x { print \"v\" }; y = x--; y", 2);
	is!("x = 3; on set x { print \"v\" }; y = (x = 7); y", 7);
}

#[cfg(feature = "native")]
#[test]
fn the_wiki_handler_runs_for_every_write_of_the_loop() {
	let output = printed("x=3\non set x {print \"x changed to \" value}\nwhile x-->0 {}\nx");
	assert!(output.contains("x changed to 2"), "{output}");
	assert!(output.contains("x changed to 0"), "{output}");
}
