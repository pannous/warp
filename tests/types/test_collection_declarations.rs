// card declaration-type: a typed declaration takes the collection words type() answers: `map data = {…}`,
// `list xs = […]`, `tuple t = (…)`, with dict and array as alias words of map and list (got-it note)
use crate::is;
use warp::normalize::{capture_hints, clear_shown_hints};

#[test]
fn a_collection_word_declares_a_variable() {
	is!("map data = {key: \"value\"}; data.key", "value");
	is!("list xs = [1 2]; xs#2", 2);
	is!("tuple t = (1, 2, 3); t#3", 3);
	is!("dict d = {a: 1}; d.a", 1);
	is!("array xs = [1 2]; xs#2", 2);
}

#[test]
fn dict_and_array_say_their_wasp_word() {
	clear_shown_hints();
	let (_, hints) = capture_hints(|| warp::wasm_emitter::eval("dict d = {a: 1}; d.a"));
	assert!(hints.iter().any(|hint| hint.original == "dict" && hint.canonical == "map"), "{hints:?}");
}

#[test]
fn a_program_naming_the_word_keeps_it() {
	is!("map = 3; map + 1", 4);
}
