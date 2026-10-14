// text * int repeats the text (Python), with a got-it warning naming `n times text`; `n times "ab"` is the explicit form.
// User decision 2026-10-03 (assumption until confirmed): also a number-like text repeats, the warning names int("5")*3.
use crate::common::fails_with;
use crate::is;
use warp::normalize::capture_hints;
use warp::wasm_emitter::eval;

#[test]
fn text_times_int_repeats() {
	is!("\"ab\"*2", "abab");
	is!("3*\"ab\"", "ababab");
	is!("greeting = \"Hello \" + \"🌍\"; greeting*2", "Hello 🌍Hello 🌍");
	is!("g=\"ab\"; n=3; x = g*n; x + \"!\"", "ababab!");
	is!("\"a\"*3", "aaa");
	is!("\"5\"*3", "555");
}

#[test]
fn n_times_text_is_the_explicit_repeat() {
	is!("2 times \"ab\"", "abab");
	is!("g=\"ab\"; 3 times g", "ababab");
}

#[test]
fn text_repeat_names_the_explicit_form() {
	let (_, hints) = capture_hints(|| eval("g=\"ab\"; g*2"));
	assert!(hints.iter().any(|hint| hint.canonical == "2 times g"), "{hints:?}");
	let (_, hints) = capture_hints(|| eval("\"5\"*3"));
	assert!(hints.iter().any(|hint| hint.reason.contains("int(\"5\")*3")), "{hints:?}");
}

#[test]
fn text_times_float_or_text_stays_a_type_error() {
	fails_with("\"ab\"*2.5", "type error");
	fails_with("\"ab\"*\"cd\"", "type error");
}
