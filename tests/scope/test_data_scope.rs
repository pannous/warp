//! Data as scope (Decided #6): a data key is a variable of its block; a hyphenated name of variables without a key subtracts
use crate::is;

#[test]
fn test_kebab_keys_are_read_back() {
	is!("a-b:2 c-d:4 a-b", 2);
	is!("a-b:2; a-b+1", 3);
	is!("a-b:2 c-d:4 c-d", 4);
}

#[test]
fn test_a_key_can_refer_to_an_earlier_key() {
	is!("{c:4 b:c}.b", 4);
	is!("p={c:4 b:c}; p.b", 4);
}

#[test]
fn test_a_key_shadows_the_subtraction_of_its_parts() {
	is!("a=5; b=1; a-b:2; a-b", 2);
}

#[test]
fn test_without_a_key_a_hyphenated_name_of_variables_subtracts() {
	is!("a=5; b=1; a-b", 4);
	is!("a=5; b=1; c=a-b; c", 4);
	is!("a=5; b=2; c=1; a-b-c", 2);
	is!("f(a,b):=a-b; f(5,1)", 4);
}

#[test]
fn test_other_hyphenated_names_stay_symbols() {
	is!("a=5; a-b", warp::Node::Symbol("a-b".to_string()));
}
