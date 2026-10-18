// The items of any tag's block (src/lowering/data/tag_children.rs): a function word takes as many items as it has
// parameters (decision function-words-consume), `name(key: v)` is the tag `name{key: v}`
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn a_call_among_a_tags_items_takes_its_parameters() {
	assert_eq!(eval("g(a: int) := a * 2; friends{g 2 \"x\"}").serialize(), eval("friends{4 \"x\"}").serialize());
}

#[test]
fn a_call_of_an_unbound_name_with_pairs_is_a_tag() {
	is!("friends{ div(class: \"x\") }.div.class", "x");
}
