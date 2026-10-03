//! `type(x)` gives the type name as a symbol (condensed from probe_type.rs; int/rational/text/inferred cases are in
//! test_todo.rs test_type)
use warp::is;
use warp::Node;

#[test]
fn test_type_symbol() {
	is!("type(hello)", Node::Symbol("symbol".to_string()));
}

#[test]
fn test_type_typed_variable() {
	is!("x:int=42;type(x)", Node::Symbol("int".to_string()));
}
