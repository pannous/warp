//! decision used-module-bare-words (card one-dispatch, notes/dispatch.md level 3): after `use json` a bare `parse(t)`
//! is json.parse; a program's own parse wins, and without the `use` parse keeps its warp meaning (the code)
use crate::is;
use warp::warp_parser::parse;

#[test]
fn a_used_modules_word_is_called_bare() {
	is!("use json; parse(\"\\\"x\\\"\")", "x");
	is!("use json; parse(\"[1, 2]\").count", 2);
	is!("use json; parse \"[1, 2]\"", parse("[1 2]"));
}

#[test]
fn without_the_use_or_with_an_own_parse_the_word_keeps_its_meaning() {
	is!("parse(\"\\\"x\\\"\")", 'x');
	is!("parse(t) := 7; use json; parse \"1\"", 7);
}
