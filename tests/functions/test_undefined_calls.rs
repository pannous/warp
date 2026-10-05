//! `name(args)` written without a space is a call: unresolved in emitted code it is an error, in data it stays data.

use crate::common::fails_with;
use warp::wasm_emitter::eval;
use warp::{is, parse_data};

#[test]
fn test_unresolved_call_is_an_error() {
	fails_with("frobnicate(3)", "undefined function: frobnicate");
	fails_with("g(1,2)", "undefined function: g");
}

#[test]
fn test_unresolved_call_in_operand_position() {
	fails_with("1+frobnicate(3)", "undefined function: frobnicate");
	fails_with("x=frobnicate(3)", "undefined function: frobnicate");
	fails_with("1.5+frobnicate(3)", "undefined function: frobnicate");
}

#[test]
fn test_unresolved_call_in_function_body() {
	fails_with("f(x):=x*2; g(3)", "undefined function: g");
	fails_with("f(x):=h(x); f(3)", "undefined function: h");
}

#[test]
fn test_unresolved_call_inside_collections() {
	fails_with("[frobnicate(3)]", "undefined function: frobnicate");
	fails_with("{a:frobnicate(3)}", "undefined function: frobnicate");
}

#[test]
fn test_variable_is_not_callable() {
	fails_with("x=2; x(3)", "undefined function: x");
}

#[test]
fn test_resolved_calls_are_unchanged() {
	is!("f(x):=x*2; f(3)", 6);
	is!("sqrt(4)", 2);
	is!("count([1 2 3])", 3);
}

#[test]
fn test_paren_data_stays_data() {
	assert_eq!(parse_data("frobnicate(3)").to_string(), "(frobnicate 3)");
	// P62 (user, 2026-10-05): brackets in code are code, an unknown word next to a value is the error; data needs `quote`
	assert!(matches!(eval("(frobnicate 3)"), warp::Node::Error(_)));
	assert!(!matches!(eval("(frobnicate, 3)"), warp::Node::Error(_)));
	assert!(matches!(eval("[frobnicate 3]"), warp::Node::Error(_)));
	assert!(!matches!(eval("quote (frobnicate 3)"), warp::Node::Error(_)));
}

#[test]
fn test_declared_type_constructs() {
	assert_eq!(eval("type P{x:int y:int}; P(1,2)").to_string(), eval("type P{x:int y:int}; P{x:1 y:2}").to_string());
}

#[test]
fn test_type_constructor_arity_is_checked() {
	fails_with("type P{x:int y:int}; P(1)", "P takes 2");
}

#[test]
fn test_print_returns_its_value_without_a_capability_error() {
	is!("print 42", 42);
	is!("print(42)", 42);
	is!("print 'hi'", "hi");
}
