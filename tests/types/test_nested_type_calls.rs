// A braceless call of a built-in type nests like a user function's (`square square 2`): `text hex 255` is
// `text(hex(255))`, `text(byte 65)` is `b = byte 65; text(b)`, one call shape for every callable
use crate::common::fails_with;
use crate::is;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

fn same(written: &str, explicit: &str) {
	assert_eq!(shown(written), shown(explicit), "{written} vs {explicit}");
}

#[test]
fn a_braceless_type_call_nests_in_a_type_call() {
	same("text hex 255", "text(hex(255))");
	same("text(byte 65)", "b = byte 65; text(b)");
	same("int float 3", "int(float(3))");
	same("text int 3.7", "text(int(3.7))");
	is!("text hex 255", "0xFF");
	is!("text(byte 65)", "0x41");
	is!("int float 3", 3);
	is!("text int 3.7", "3");
}

#[test]
fn a_braceless_type_call_nests_like_a_user_function() {
	is!("square(n) := n*n; square square 2", 16);
	is!("f(x) := x*2; text f 4", "8");
	is!("f(x) := x*2; f int 3.7", 6);
}

#[test]
fn a_type_named_by_the_program_keeps_its_meaning() {
	// decision type-words-reserved: a conversion word names no function
	fails_with("hex(x) := x * 2; text hex 4", "hex is a type");
}

#[test]
fn a_braceless_type_call_compares_like_a_user_function() {
	is!("int 3 == 3", true);
	is!("int 3.7 == 3", true);
	is!("hex 255 == 255", true);
	is!("int 3 == 4", false);
	is!("f(x) := x*2; f 3 == 6", true);
	same("int 3.7 == 3", "(int 3.7) == 3");
}

#[test]
fn a_braceless_truth_type_call_next_to_a_comparison_is_ambiguous_like_an_untyped_function() {
	crate::common::fails_with("same(x) := x; same 3 == 9", "write (same 3) == 9 or same(3 == 9)");
	crate::common::fails_with("bool 3 == 9", "write (bool 3) == 9 or bool(3 == 9)");
}
