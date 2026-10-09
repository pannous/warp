// card byte-slice: an Error is no text: the text builtins (byte_slice, byte_at, trim, starts_with, ends_with) and .bytes
// fail with the error's own message, as `e + 1` does, instead of reading the message as text
use crate::is;
use warp::error;

#[test]
fn a_text_builtin_of_an_error_fails_with_the_error() {
	is!("e = error(\"boom\"); byte_slice(e, 0, 2)", error("boom"));
	is!("e = error(\"boom\"); byte_at(e, 1)", error("boom"));
	is!("e = error(\"boom\"); e.bytes", error("boom"));
	is!("e = error(\"boom\"); trim(e)", error("boom"));
	is!("e = error(\"boom\"); starts_with(e, \"b\")", error("boom"));
	is!("t = \"boom\"; byte_slice(t, 0, 2)", "bo");
	is!("t = \"boom\"; t.bytes", 4);
}
