use warp::*;

#[test]
fn test_type_of_list_names_its_element_type() {
	is!("type([1 2 3])", "list of int");
	is!("pixels=(1,2,3);type(pixels)", "list of int");
	is!("type([\"ab\" \"cd\"])", "list of text");
}

#[test]
fn test_type_of_mixed_list_is_plain_list() {
	is!("type([1 \"ab\"])", "list");
}

#[test]
fn test_plural_type_word_denotes_a_list() {
	is!("x:ints=[1 2 3];type(x)", "list of int");
	is!("x:numbers=[1 2 3];type(x)", "list of number");
	is!("x:numbers=[1 2 3];x#2", 2);
}

#[test]
fn test_data_key_is_a_scope() {
	is!("a-b:2 c-d:4 a-b", 2);
	is!("a-b:2 c-d:4 c-d", 4);
}

#[test]
fn test_kebab_key_that_is_also_subtraction_warns() {
	let warnings = warp::analyzer::lint(&warp::parse("a=5; b=3; a-b:2 c-d:4 a-b"));
	assert!(warnings.iter().any(|warning| warning.message.contains("a-b")), "{warnings:?}");
	assert!(warp::analyzer::lint(&warp::parse("a-b:2 c-d:4 a-b")).is_empty());
}
