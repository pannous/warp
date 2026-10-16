//! Doc strings (card doc-strings, decision reflection): a text literal as the first statement of a function body is
//! its doc and is not evaluated; the comment right above a definition is its doc too. `f.doc` and `help(f)` show it
use crate::is;

#[test]
fn a_leading_text_is_the_doc_and_not_evaluated() {
	is!("fun twice(x) {\"doubles a number\"; x*2}\ntwice(3)", 6);
	is!("fun twice(x) {\n  \"doubles a number\"\n  x*2\n}\ntwice.doc", "doubles a number");
	is!("twice(x) := {\"doubles a number\"; x*2}\ntwice.doc", "doubles a number");
}

#[test]
fn the_comment_above_a_definition_is_the_doc() {
	is!("# doubles a number\nfun twice(x) { x*2 }\ntwice.doc", "doubles a number");
	is!("// doubles a number\ntwice(x) := x*2\ntwice.doc", "doubles a number");
	is!("# doubles a number\ntwice(x) := x*2\nhelp(twice)", "twice(x) -> int\ndoubles a number");
}

#[test]
fn help_shows_the_doc_below_the_signature() {
	is!("# doubles a number\nfun twice(x) { x*2 }\nhelp(twice)", "twice(x) -> int\ndoubles a number");
	is!("fun twice(x) {\"doubles a number\"; x*2}\nhelp(twice)", "twice(x) -> int\ndoubles a number");
}

#[test]
fn a_lone_text_body_stays_the_value() {
	is!("fun hi() {\"hello\"}\nhi()", "hello");
}

#[test]
fn a_function_without_doc_has_the_empty_doc() {
	is!("twice(x) := x*2\ntwice.doc", "");
}
