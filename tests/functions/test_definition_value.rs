// A program that only defines a function has the value ø (it used to fail WASM validation or call the head)
use warp::*;

#[test]
fn a_definition_alone_is_empty() {
	is!("square := it*it", Empty);
	is!("f(x) := x + 1", Empty);
	is!("square = {it*it}", Empty);
	is!("square := it*it; square 4", 16);
}
