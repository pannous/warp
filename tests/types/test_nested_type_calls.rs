// A braceless call of a built-in type nests like a user function's (`square square 2`): `text hex 255` is
// `text(hex(255))`, `text(byte 65)` is `b = byte 65; text(b)`, one call shape for every callable
use warp::wasm_emitter::eval;
use crate::is;

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
	is!("hex(x) := x * 2; text hex 4", "8");
}
