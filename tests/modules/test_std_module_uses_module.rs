//! A standard module using another one (card std-module-uses-module, notes/stdlib.md §8): lib/text.warp's camel_case
//! calls list's drop
use crate::is;
use warp::warp_parser::parse;

#[test]
fn a_std_module_calls_another_modules_word() {
	is!("use text; camel_case(\"\")", "");
}

#[test]
fn the_used_modules_words_stay_hidden_from_the_program() {
	// list's words are standard functions anyway (decision type-library-words); a selective import still hides the rest
	is!("use text; take([1, 2, 3], 2)", parse("[1 2]"));
	crate::common::fails_with(
		"from list import zip; unique([1, 1])",
		"unique is in the standard module list: write `use list`",
	);
}

#[test]
fn the_program_uses_both_modules() {
	is!("use text; use list; [camel_case(\"a b\"), drop([1, 2], 1)]", parse(r#"["aB" [2]]"#));
}

#[test]
fn a_programs_own_word_of_the_hidden_name_is_its_own() {
	is!("use text; drop(x) := x * 2; [camel_case(\"a b\"), drop(4)]", parse(r#"["aB" 8]"#));
}
