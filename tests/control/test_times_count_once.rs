// #22 (user 2026-10-03, "N once, keep rest"): `N times {…}` evaluates N once; a trailing `i++ while c` stays a plain
// while (with a hint), `a = 2 if c` guards the whole assignment.
use crate::is;
use warp::normalize::capture_hints;

#[test]
fn times_evaluates_its_count_once() {
	is!("n=0; k=3; k times {n+=1; k=1}; n", 3);
	is!("n=0; k=2; k times {n+=1; k+=1}; n", 2);
	is!("n=0; 3 times {n+=1}; n", 3);
}

#[test]
fn trailing_while_stays_a_plain_while_with_a_hint() {
	is!("i=5; i++ while i<3; i", 5);
	is!("i=0; i++ while i<3; i", 3);
	let hints = capture_hints(|| warp::wasm_emitter::eval("i=5; i++ while i<3; i")).1;
	assert!(hints.iter().any(|hint| hint.canonical.starts_with("while ")), "{hints:?}");
}

#[test]
fn a_trailing_if_guards_the_whole_assignment() {
	is!("a=1; a = 2 if 0; a", 1);
	is!("a=1; a = 2 if 1; a", 2);
}
