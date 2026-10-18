//! decisions.md markup-plain-library: `name = value` inside a data block is the entry `name: value`, also beside the
//! block's content and for a keyword name; a block holding statements stays code (src/lowering/data/tag_entries.rs)
use crate::is;
use warp::wasm_emitter::eval;
use warp::web::markup::to_html;

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

#[test]
fn an_assignment_beside_content_is_an_entry() {
	assert_eq!(
		html_of("a{href=\"/about\" data-page=\"about\" \"About\"}"),
		"<a href=\"/about\" data-page=\"about\">About</a>"
	);
	is!("cat{ x = 1 \"t\" }; cat.x", 1);
}

#[test]
fn a_keyword_names_an_entry() {
	assert_eq!(html_of("p{class=\"card\" \"x\"}"), "<p class=\"card\">x</p>");
	assert_eq!(eval("Person{ class = \"a\" }").serialize(), eval("Person{ class: \"a\" }").serialize());
}

#[test]
fn a_block_of_statements_stays_code() {
	is!("x = 0\nif x == 0 { x = 5 }\nx", 5);
	is!("f := { y = 2; y * 3 }\nf()", 6);
}
