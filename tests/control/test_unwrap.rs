//! `x!` unwraps (wiki/optional.md, Error.md, D2 by position: after a plain variable): the value, or a loud error
//! when it is ø or an Error. Errors are values (Decided #1): unwrapping ø raises "unwrapped ø", catchable by `try`.
use crate::common::fails_with;
use warp::is;

#[test]
fn test_unwrap_gives_the_value() {
	is!("x=3; x!", 3);
	is!("x=3; x!+1", 4);
	is!("x:int?=5; x!*2", 10);
	is!("x=\"a\"; x!", "a");
}

#[test]
fn test_unwrap_of_nothing_or_an_error_is_loud() {
	fails_with("x=ø; x!", "unwrapped ø");
	fails_with("ø!", "unwrapped ø");
	// in arithmetic the declared Int fails first (loud, though not yet with the unwrap message)
	assert!(matches!(warp::wasm_emitter::eval("x:int?=ø; x!+1"), warp::Node::Error(_)));
	fails_with("x=error(\"bad\"); x!", "bad");
}

#[test]
fn test_mutating_and_evaluating_bangs_keep_their_meaning() {
	is!("x=\"hi\"; x.upper!; x", "HI"); // D2: after a method it mutates
	is!("{1+2}!", 3); // after a block it evaluates
}
