//! A standard module's word called through the module's name: `list.zip(a, b)` is `zip(a, b)` (notes/stdlib.md Q2)
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn a_qualified_module_word_is_the_word() {
	is!("use list; list.zip([1], [2])", parse("[[1 2]]"));
	is!("use math; math.gcd(4, 6)", 2);
	is!("use text; text.repeat(\"a\", 2)", "aa");
	is!("math.sqrt(16)", 4);
	is!("Math.sqrt(16)", 4);
}

#[test]
fn a_variable_of_the_modules_name_keeps_its_methods() {
	is!("text = \"ab\"; text.upper()", "AB");
	is!("list = [3, 1]; list.sort()", parse("[1 3]"));
}

#[test]
fn a_qualified_word_without_its_use_names_the_module() {
	crate::common::fails_with("list.zip([1], [2])", "zip is in the standard module list: write `use list`");
}

#[test]
fn a_std_word_without_its_use_shows_no_hint_of_the_std_source() {
	let (_, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval("list.zip([1],[2])"));
	let written: Vec<String> = hints.iter().map(|hint| hint.original.clone()).collect();
	// the note on the qualified call is the program's own; lib/map.wasp's `out[k]` is not
	assert!(written.iter().all(|original| original.starts_with("list.zip")), "{written:?}");
}
