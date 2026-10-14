//! `type of x` is `type(x)` for every x (card type-of-consistency, issue #1011): functions give their function type,
//! type words the type `type`, maps their map type; a bare parameterless function is called (decision reflection)
use crate::is;
use warp::Node;

fn symbol(name: &str) -> Node {
	Node::Symbol(name.to_string())
}

#[test]
fn test_type_of_a_user_function() {
	is!("def a(x):=x; type of a", "function: (x) -> int");
	is!("f:=it*2; type of f", "function: (it) -> int");
}

#[test]
fn test_type_of_a_library_function() {
	is!("use math; type of gcd", "function: (a, b) -> int");
}

#[test]
fn test_type_of_a_type_word() {
	is!("type of int", symbol("type"));
	is!("type(int)", symbol("type"));
}

#[test]
fn test_type_of_a_map() {
	is!("type of {a:1}", symbol("map of int"));
}

#[test]
fn test_type_of_a_parameterless_function_calls_it() {
	is!("def a():=1; type of a", symbol("int"));
	is!("def a():=1; type(a)", symbol("int"));
	is!("def a():=1; type function a", "function: () -> int");
}

#[test]
fn test_type_of_values_and_unknown_names_unchanged() {
	is!("type of 3", symbol("int"));
	is!("type of [1 2]", symbol("list of int"));
	is!("type of foo", symbol("symbol"));
}
