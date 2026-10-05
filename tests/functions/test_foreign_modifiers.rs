//! P78 (user 2026-10-05): modifier words of other languages without wasp meaning (`public`, `static`, `virtual` …) are
//! skipped with the note "public has no meaning in wasp"; `global` and `const` keep their meaning
use warp::diagnostic::take_warnings;
use warp::is;

#[test]
fn test_meaningless_modifiers_are_skipped_with_a_note() {
	take_warnings();
	is!("public static fun f(){3}; f()", 3);
	assert!(take_warnings().iter().any(|warning| warning.message.contains("public has no meaning in wasp")));
	is!("private x = 4; x + 1", 5);
	is!("public def g(x){x*2}; g(5)", 10);
}

#[test]
fn test_meaningful_modifiers_keep_their_meaning() {
	is!("export fun f(){3}; f()", 3);
	is!("global fun f(){3}; f()", 3);
	is!("const k = 7; k", 7);
	is!("public = 3; public + 1", 4);
}
