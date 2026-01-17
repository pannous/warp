use warp::{is, Node};
use warp::node::types;
// @claude once tests here are passing, move them to the appropriate correct test file!

#[test]
#[ignore = "GRAFIX test - wasm_paint_routine not defined"]
#[cfg(feature = "GRAFIX")]
fn test_paint_wasm() {
	is!("h=100;r=10;i=100;c=99;r=99;x=i%w;y=i/h;k=‖(x-c)^2+(y-c)^2‖<r", 1);
}

// === If-then-else ===
#[test]
fn test_if_then_else() {
	is!("if 4>1 then 2 else 3", 2);
}

// === List indexing after puts ===
#[test]
fn test_list_index_after_puts() {
	is!("puts('ok');(1 4 3)#2", 4);
}

// === Square root arithmetic ===
#[test]
fn test_sqrt_arithmetic() {
	is!("3 + √9", 6);
}

// === Lambda/closure tests ===
#[test]
fn test_lambda_simple() {
	is!("grows:=it*2; grows 3", 6);
}

#[test]
fn test_lambda_with_multiply() {
	is!("grows:=it*2; grows 3*4", 24);
}

#[test]
fn test_lambda_comparison() {
	is!("grows:=it*2; grows(3*42) > grows 2*3", 1);
}

// === $0 parameter reference ===
#[test]
fn test_param_reference() {
	// $0 parameter reference (explicit param style with parentheses)
	is!("add1(x):=$0+1;add1(3)", 4);
}

// === Index assignment in loops (now working!) ===
#[test]
fn test_index_assign_in_loop() {
	// Index assignment with properly sized array
	is!("i=0;pixel=(0 0 0 0 0);while(i++<5){pixel[i]=i%2};i", 5);
}


#[test]
fn test_type() {
	// type() returns a Symbol with the type name
	is!("type(42)", Node::Symbol("int".to_string()));
	is!("type(3.14)", Node::Symbol("float".to_string()));
	is!("type('hello')", Node::Symbol("text".to_string()));
	// Type of inferred variable
	is!("x=42;type(x)", Node::Symbol("int".to_string()));
}


#[test]
fn test_type_equivalence() {
	is!("x:int=1;type(x)",Node::Symbol("int".to_string()));
	is!("x:int=1;type(x)",types("int")); // via above
	// is!("x:int=1;type(x)","int"); // via above
}

#[test]
fn test_type_annotated() {
	// Explicit type annotation - type tracking now implemented in scope
	is!("x:int=1;type(x)", Node::Symbol("int".to_string()));
}


#[test]
#[ignore = "todo"]
fn test_array_type_generics() {
	is!("pixels=(1,2,3);type(pixels)","list<int>");
}



#[test]
fn test_array_length() {
	is!("pixels=(1,2,3);#pixels", 3); // element count ⚠️ number operator ≠ index ≠ comment
	is!("pixels=(1,2,3);count(pixels)", 3); // element count
	is!("pixels=(1,2,3);pixels.count()", 3); // element count
	is!("pixels=(1,2,3);pixels.count", 3); // element count methods/getters don't need ()
	// is!("pixels=(1,2,3);pixel count", 3); // element count
	// is!("pixels=(1,2,3);number of pixels", 3); // element count - requires natural language "of" syntax
	is!("pixels=(1,2,3);pixels.number()", 3); // element count
	is!("pixels=(1,2,3);size(pixels) ", 3 * 8); // ⚠️ byte count as i64
	// is!("pixels=(1,2,3);length(pixels) ", 3 * xyz); // ⚠️ byte count as node ???
}


#[test]
#[ignore = "typed array constructor not yet implemented"]
fn test_array_constructor() {
	is!("i=0;w=800;h=800;pixels=640000*int;size(pixels) ", 800 * 800 * 4); // byte count
	is!("i=0;w=800;h=800;pixels=640000*int;length(pixels) ", 800 * 800); // element count
}

// === Still pending (requires major features) ===
#[test]
#[ignore = "requires polymorphic function dispatch"]
fn test_polymorphic_dispatch() {
	is!("square(3.0)", 9.);
}

#[test]
#[ignore = "requires print function implementation"]
fn test_print_function() {
	is!("print 3", 3);
}

#[test]
#[ignore = "requires UTF-8 char indexing vs byte indexing"]
fn test_utf8_char_indexing() {
	// UTF-8 char indexing vs byte indexing (encoding redesign)
	is!("'αβγδε'#3", 'γ');
}
