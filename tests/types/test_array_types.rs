//! Typed and fixed arrays: `x : 100 int`, `pixel:int[100]`, `char[3]`, `26 * char` are zero-filled lists; `[number]` is `list of number`
use crate::is;

#[test]
fn test_typed_array_as_a_value() {
	is!("letters = char[3]; count letters", 3);
	is!("x = int[4]; count x", 4);
	is!("x = int[4]; x#1", 0);
	is!("x = int[4]; x#2=7; x#2", 7);
	is!("x = float[2]; count x", 2);
}

#[test]
fn test_count_times_type_is_a_typed_array() {
	is!("upcases = 26 * char; count upcases", 26);
	is!("x = 3 * int; count x", 3);
	is!("x = 3 * int; x#3", 0);
}

#[test]
fn test_declared_typed_arrays_keep_working() {
	is!("x : 100 int; count x", 100);
	is!("x : 100 int; x#1", 0);
	is!("pixel:int[100]; pixel#1=5; pixel#1", 5);
	is!("pixel:int[100]; pixel.length", 100);
}

#[test]
fn test_bracketed_type_is_a_list_of_that_type() {
	is!("x:[number]=[1 2]; type(x)", "list of number");
	is!("x:[int]=[1 2]; type(x)", "list of int");
	is!("x:[number]=[1 2]; count x", 2);
}

#[test]
fn test_plural_declarations_are_lists() {
	is!("numbers x = [1 2]; count x", 2);
	is!("numbers x = [1 2]; type(x)", "list of number");
	is!("x:numbers=[1 2]; type(x)", "list of number");
}

mod hints {
	use warp::normalize::*;
	use warp::warp_parser::WarpParser;

	#[test]
	fn test_bracketed_list_type_hints_the_plural_word() {
		set_hint_mode(HintMode::Always);
		let (_, hints) = capture_hints(|| WarpParser::parse("x:[number]=[1 2]"));
		let spelled: Vec<(String, String)> = hints.into_iter().map(|hint| (hint.original, hint.canonical)).collect();
		assert_eq!(spelled, vec![("[number]".to_string(), "numbers".to_string())]);
	}

	#[test]
	fn test_plural_word_needs_no_hint() {
		set_hint_mode(HintMode::Always);
		let (_, hints) = capture_hints(|| WarpParser::parse("x:numbers=[1 2]"));
		assert!(hints.is_empty(), "{hints:?}");
	}
}

#[test]
fn test_declaration_with_count_times_type() {
	is!("x : 100 * int; count x", 100);
	is!("x : 100 * ints; count x", 100);
	is!("x : 3 * char; count x", 3);
	is!("x : 100 numbers; count x", 100);
}
