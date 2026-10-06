//! A struct built by its constructor has readable fields, also directly or through a variable

use crate::is;

#[test]
fn a_field_of_a_constructed_struct_variable() {
	is!("struct point{x:int y:int}; p=point(1,2); p.y", 2);
	is!("struct point{x:int y:int}; p=point(1,2); p.x", 1);
}

#[test]
fn a_field_of_a_constructor_call() {
	is!("struct point{x:int y:int}; point(1,2).y", 2);
}

#[test]
fn a_field_in_arithmetic() {
	is!("struct point{x:int y:int}; p=point(1,2); p.y+p.x", 3);
	is!("p={x:1 y:2}; p.y", 2);
}

#[test]
fn a_missing_field_is_an_error_value() {
	match warp::wasm_emitter::eval("struct point{x:int y:int}; p=point(1,2); p.z") {
		warp::Node::Error(_) => {}
		other => panic!("expected an error, got {other:?}"),
	}
}
