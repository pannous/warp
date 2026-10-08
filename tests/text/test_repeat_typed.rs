// text * int is text at compile time: a variable declared int refuses it before the program runs (card repeat-int,
// found by the type model W0)
use crate::common::fails_with;
use crate::is;
use warp::law::type_model::warp_verdict;

#[test]
fn repeated_text_does_not_fit_int() {
	for code in ["x: int = \"5\"*3; x", "x: int = 3*\"ab\"; x", "x: float = \"ab\"*2; x"] {
		assert!(warp_verdict(code).is_err(), "{code} compiles");
		fails_with(code, "is declared");
	}
}

#[test]
fn repeated_text_fits_text() {
	is!("x: text = \"5\"*3; x", "555");
	is!("x: text = 2*\"ab\"; x", "abab");
}
