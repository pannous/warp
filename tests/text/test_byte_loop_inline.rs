//! A byte loop reads its text without calls (card loop-calls): `byte_at(t, i)` and `t.bytecount` of a text are inline,
//! a character or anything else still goes through the runtime
use crate::is;

#[test]
fn a_byte_loop_over_a_text_parameter() {
	is!(
		"total(t) := { s = 0; i = 0; while i < t.bytecount { s = s + byte_at(t, i); i = i + 1 }; s }; total(\"abc\")",
		294
	);
	is!(
		"fast total(t) := { s = 0; i = 0; while i < t.bytecount { s = s + byte_at(t, i); i = i + 1 }; s }; total(\"é\")",
		364
	);
}

#[test]
fn a_character_is_read_as_its_text() {
	is!("byte_at('a', 0)", 97);
	is!("c = 'é'; byte_at(c, 1)", 0xA9);
	is!("c = 'é'; c.bytecount", 2);
	is!("[1, 2].bytecount", 16);
}

#[test]
fn an_inline_byte_read_stays_in_bounds() {
	is!("f(t) := byte_at(t, t.bytecount); f(\"ab\")", warp::error("index out of range at 1:34"));
}
