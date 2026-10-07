//! Other ecosystems' names for wasp's standard words (notes/stdlib.md, alias rule): `JSON.parse(t)` and `json.loads(t)`
//! are parse_json(t) with a got-it note, and bring their module (`use json`) themselves; arguments in the other
//! order are put in wasp's (`re.findall(pattern, text)` is find_all(text, pattern))
use crate::is;
use warp::normalize::{capture_hints, clear_shown_hints};
use warp::wasp_parser::parse;

#[test]
fn foreign_standard_names_are_wasp_words() {
	is!("JSON.parse(\"[1, 2]\")", parse("[1 2]"));
	is!("json.loads(\"[1, 2]\")", parse("[1 2]"));
	is!("JSON.stringify([1, 2])", "[1,2]");
	is!("json.dumps([1, 2])", "[1,2]");
	is!("re.findall(\"[0-9]+\", \"a11 b22\")", parse("[\"11\" \"22\"]"));
	is!("re.sub(\"[0-9]+\", \"#\", \"a1 b22\")", "a# b#");
	is!("os.getenv(\"WARP_TEST_NO_SUCH_VARIABLE\")", parse("ø"));
}

#[test]
fn a_foreign_name_says_the_wasp_word() {
	clear_shown_hints();
	let (_, hints) = capture_hints(|| warp::wasm_emitter::eval("JSON.parse(\"[1]\")"));
	assert!(hints.iter().any(|hint| hint.original == "JSON.parse" && hint.canonical == "parse_json"), "{hints:?}");
}

#[test]
fn a_program_naming_the_module_keeps_it() {
	is!("re = 3; re + 1", 4);
	is!("JSON = {parse: 5}; JSON.parse", 5);
}
