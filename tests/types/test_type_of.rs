//! `type(x)` gives the type name as a symbol (condensed from probe_type.rs; int/rational/text/inferred cases are in
//! test_todo.rs test_type)
use crate::is;
use warp::Node;

#[test]
fn test_type_symbol() {
	is!("type(hello)", Node::Symbol("symbol".to_string()));
}

#[test]
fn test_type_typed_variable() {
	is!("x:int=42;type(x)", Node::Symbol("int".to_string()));
}

#[test]
fn test_type_inside_a_function_body() {
	is!("def f(x){ type(x) }; f(3)", Node::Symbol("int".to_string()));
	is!("def f(x){ type(x) }; f(\"ab\")", Node::Symbol("text".to_string()));
	is!("def f(x){ y=type(x); y }; f(3)", Node::Symbol("int".to_string()));
}

#[test]
fn test_type_as_a_method() {
	let symbol = |name: &str| Node::Symbol(name.to_string());
	is!("x=3; x.type", symbol("int"));
	is!("x=3; x.type()", symbol("int"));
	is!("\"ab\".type", symbol("text"));
	is!("x=3; x.type == type(x)", true);
	// a field named type stays the field
	is!("e = {type: \"click\"}; e.type", "click");
}
