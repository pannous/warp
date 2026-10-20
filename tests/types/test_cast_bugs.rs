use crate::is;
use warp::wasm_emitter::eval;

fn text_of(code: &str) -> String {
	eval(code).serialize()
}

fn loud_cast_error(code: &str) {
	let result = format!("{:?}", eval(code));
	assert!(result.contains("cannot cast"), "{code}: {result}");
}

#[test]
fn a_cast_to_char_can_be_stored() {
	assert_eq!(text_of("x=65 as char; x"), "'A'");
	assert_eq!(text_of("65 as char"), "'A'");
	is!("ord('A')", 65);
	is!("x=65 as char; ord(x)", 65);
}

#[test]
fn a_one_character_text_can_be_indexed() {
	assert_eq!(text_of("\"a\"#1"), "'a'");
	assert_eq!(text_of("\"abc\"#2"), "'b'");
	assert_eq!(text_of("x=\"a\"; x#1"), "'a'");
	assert!(text_of("\"a\"#2").contains("index out of range"));
}

#[test]
fn a_text_as_list_is_its_characters() {
	assert_eq!(text_of("\"abc\" as list"), "['a' 'b' 'c']");
	assert_eq!(text_of("\"a\" as list"), "['a']");
	assert_eq!(text_of("[1 2] as list"), "[1 2]");
}

#[test]
fn a_runtime_value_as_string() {
	is!("x=3; x as string", "3");
	is!("x=\"hi\"; x as string", "hi");
	is!("x=3 as string; x", "3");
	is!("x=[1 2]; x as string", "[1 2]"); // user decision #35
}

#[test]
fn nonsense_casts_are_loud() {
	loud_cast_error("3 as list");
	loud_cast_error("[1 2] as int");
	loud_cast_error("[1 2] as float");
	loud_cast_error("[1 2] as char");
}

#[test]
fn a_parameter_as_list_takes_lists_and_texts() {
	is!("l(x) := x as list; 3", 3); // an uncalled definition compiles
	assert_eq!(text_of("l(x) := x as list; l([1 2]); l(\"ab\")"), "['a' 'b']");
	assert_eq!(text_of("l(x) := x as list; l(\"ab\"); l([1 2])"), "[1 2]");
	is!("l(x) := x as list; #l(\"abc\") + #l([1 2])", 5);
}

// card cast-list: a cast to a list type converts each item; an empty list is an empty list of that type
#[test]
fn a_cast_to_a_list_type_converts_each_item() {
	is!("xs = [1, 2] as floats; xs#1 + 0.5", 1.5);
	is!("xs = [1, 2] as [float]; xs#2", 2.0);
	is!("xs = [1, 2] as list of float; xs#2", 2.0);
	is!("ys = [\"1\", \"2\"]; zs = ys as ints; zs#2 + 1", 3);
	is!("xs = [] as list of int; count xs", 0);
	is!("xs = [] as [int]; count xs", 0);
	is!("xs = [] as ints; count xs", 0);
}
