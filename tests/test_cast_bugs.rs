use warp::wasm_emitter::eval;
use warp::is;

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
	is!("'A' as int", 65);
	is!("x=65 as char; x as int", 65);
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
	let list_variable = format!("{:?}", eval("x=[1 2]; x as string"));
	assert!(list_variable.contains("cannot cast list to string"), "{list_variable}");
}

#[test]
fn nonsense_casts_are_loud() {
	loud_cast_error("3 as list");
	loud_cast_error("[1 2] as int");
	loud_cast_error("[1 2] as float");
	loud_cast_error("[1 2] as char");
}
