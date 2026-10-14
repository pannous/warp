//! card keep-list: a list keeps its separator (`,` `;` newline, space) in the wasm Node, so warp code reads it
//! (`n.separator`), builds it (`separated(list, ";")`) and the list read back is written as it was
use crate::is;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_list_shows_its_separator() {
	is!("c = data [a, b]; c.separator", ",");
	is!("c = data (a; b); c.separator", ";");
	is!("c = data [a b]; c.separator", " ");
	is!("c = data [a]; c.separator", "");
}

#[test]
fn a_list_read_back_keeps_its_separator() {
	assert_eq!(shown("x = data [a, b]; if clock() > 0 then x else 0"), "[a, b]");
	assert_eq!(shown("x = data {a; b}; if clock() > 0 then x else 0"), "{a; b}");
}

#[test]
fn a_list_is_built_with_a_separator() {
	assert_eq!(shown("separated(bracketed([1, 2], \"(\"), \";\")"), "(1; 2)");
	is!("separated([1, 2], \"|\")", warp::error("unknown separator: |"));
}
