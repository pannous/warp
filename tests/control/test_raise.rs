// `raise X` / `throw X` (wiki Error.md, Exception.md): an exception that ends the run with its message unless a `try`
// catches it; `raise error("…")` raises that error's message
use crate::common::fails_with;
use warp::is;

#[test]
fn raise_ends_the_run_with_its_message() {
	fails_with("raise \"boom\"", "boom");
	fails_with("x = 1; raise \"stop\"; x", "stop");
	fails_with("raise error(\"bad\")", "bad");
	fails_with("f(x) := if x < 0 then raise \"negative\" else x; f(-1)", "negative");
	is!("f(x) := if x < 0 then raise \"negative\" else x; f(4)", 4);
}

#[test]
fn try_catches_a_raise() {
	is!("try raise \"boom\" else 3", 3);
	is!("try throw \"boom\" else 2", 2);
	is!("f(x) := if x < 0 then raise \"negative\" else x; try f(-1) else 7", 7);
}
