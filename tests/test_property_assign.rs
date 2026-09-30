//! Assignment to a field (value semantics like `x#i=v`) and safe navigation `?.`
use warp::wasm_emitter::eval;
use warp::*;
mod common;
use common::fails_with;

fn printed(code: &str) -> String {
	eval(code).serialize().replace('\'', "\"")
}

#[test]
fn test_assign_a_field() {
	is!("p={a:1}; p.a=2; p.a", 2);
	is!("p={a:1 b:2}; p.b=5; p.a + p.b", 6);
	is!("p={a:1 b:2}; p[\"a\"]=5; p.a", 5);
	assert_eq!(printed("p={a:1 b:2}; p.a=7; p"), "{a:7 b:2}");
}

#[test]
fn test_assign_a_nested_field() {
	is!("p={a:1 b:{c:3}}; p.b.c=4; p.b.c", 4);
	is!("p={a:1 b:{c:3}}; p.b.c=4; p.a", 1);
	is!("p={a:1 b:{c:3 d:5}}; p.b.c=4; p.b.d", 5);
	is!("p={a:1 b:{c:3 d:5}}; q=p; p.b.c=4; q.b.c", 3);
}

#[test]
fn test_assignment_has_value_semantics() {
	is!("p={a:1 b:2}; q=p; p.a=9; q.a", 1);
	is!("p={a:1 b:2}; q=p; p.a=9; p.a", 9);
}

#[test]
fn test_assigning_a_new_field_adds_it() {
	is!("p={a:1 b:2}; p.z=9; p.z", 9);
	assert_eq!(printed("p={a:1 b:2}; p.z=9; p"), "{a:1 b:2 z:9}");
	is!("p={a:1}; p.z=9; p.a + p.z", 10);
}

#[test]
fn test_assigning_a_field_of_a_non_object_is_an_error() {
	fails_with("x=1; x.a=2", "undefined function: a");
	fails_with("x=[1 2]; x.a=2", "undefined function: a");
}

#[test]
fn test_safe_navigation() {
	is!("{a:1}?.a", 1);
	is!("p={a:1 b:2}; p?.b", 2);
	assert_eq!(printed("x=ø; x?.a"), "ø");
	fails_with("p={a:1}; p?.zz", "no field zz");
}

#[test]
fn test_a_one_entry_object_counts_one_pair() {
	is!("p={a:1}; p.length", 1);
	is!("p={a:1}; count p", 1);
	is!("p={a:1 b:2}; count p", 2);
}
