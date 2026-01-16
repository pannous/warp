use warp::is;
use warp::Node;

#[test]
fn test_type_number() {
	is!("type(42)", Node::Symbol("int".to_string()));
}

#[test]
fn test_type_float() {
	is!("type(3.14)", Node::Symbol("float".to_string()));
}

#[test]
fn test_type_string() {
	is!("type('hello')", Node::Symbol("text".to_string()));
}

#[test]
fn test_type_variable() {
	// Type of a variable reflects the inferred type
	is!("x=42;type(x)", Node::Symbol("int".to_string()));
}

#[test]
fn test_type_symbol() {
	// Type of a bare symbol is "symbol"
	is!("type(hello)", Node::Symbol("symbol".to_string()));
}

#[test]
#[ignore = "typed variable declaration tracking not yet implemented"]
fn test_type_typed_variable() {
	// Explicit type annotation - needs type tracking in scope
	is!("x:int=42;type(x)", Node::Symbol("int".to_string()));
}
