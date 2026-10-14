//! Card print-end (user): Ruby's `for x in 0..3; print x; end` loops over the statements up to `end`, with a hint
//! naming the braces form
use crate::is;
use warp::normalize::{capture_hints, clear_shown_hints};
use warp::wasm_emitter::eval;

#[test]
fn the_statements_up_to_end_are_the_body() {
	is!("s = 0; for x in 1..3; s += x; end; s", 3);
	is!("s = 0; for x in 1..3; s += x; s += 1; end; s", 5);
	is!("n = 0\nfor x in 1..4;\n  n += 1\nend\nn", 3);
}

#[test]
fn the_semicolon_end_form_hints_the_braces() {
	clear_shown_hints();
	let (_, hints) = capture_hints(|| eval("s = 0; for x in 1..3; s += x; end; s"));
	assert!(hints.iter().any(|hint| hint.canonical.contains('{')), "{hints:?}");
}
