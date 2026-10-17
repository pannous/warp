// card type-values (decisions type-print, type-text): a type value is `class int` in every text form, printed, joined
// and interpolated, so pasted back it is the type again (class_references.rs); its own type is type
use crate::is;
use warp::normalize::capture_hints;

fn shown(program: &str) -> String {
	warp::wasm_emitter::eval(program).serialize()
}

#[test]
fn a_type_value_shows_as_its_class() {
	assert_eq!(shown("type 3"), "class int");
	assert_eq!(shown("int"), "class int");
	assert_eq!(shown("x = int; x"), "class int");
	assert_eq!(shown("class Person{name}; Person"), "class Person");
	assert_eq!(shown("[int, text]"), "[class int class text]");
}

#[test]
fn a_type_value_joins_as_its_class() {
	is!("\"type is \" + type 3", "type is class int");
	is!("x = int; \"\\(x)\"", "class int");
	is!("x = int; x as text", "class int");
	is!("x = 3; \"\" + type(x) == \"class int\"", true);
}

#[test]
fn the_type_of_a_type_value_is_type() {
	is!("x = int; type x", "type");
	is!("type(int)", "type");
	is!("f(t: type) := type(t); f int", "type");
}

#[test]
fn a_type_value_still_compares_as_the_type() {
	is!("x = 3; type(x) == int", true);
	is!("x = int; x == int", true);
}

#[test]
fn a_joined_type_compared_with_its_name_hints_the_type_word() {
	let hints = capture_hints(|| warp::wasm_emitter::eval("x = 3; \"\" + type(x) == \"int\"")).1;
	assert!(hints.iter().any(|hint| hint.canonical == "type(x) == int"), "{hints:?}");
}
