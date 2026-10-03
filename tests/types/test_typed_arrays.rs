use warp::*;
use crate::common::fails_with;

#[test]
fn test_typed_array_declaration_with_count_first() {
	is!("x : 100 int; x.length", 100);
	is!("x : 100 int; x#1", 0);
	is!("x : 100 int; x#5=7; x#5", 7);
}

#[test]
fn test_typed_array_declaration_with_subscript() {
	is!("pixel:int[100]; pixel.length", 100);
	is!("pixel:int[100]; pixel#1", 0);
	is!("pixel:int[100]; pixel#5=7; pixel#5", 7);
}

#[test]
fn test_typed_array_of_other_types() {
	is!("x : 10 float; x.length", 10);
	is!("x : 3 text; x.length", 3);
	is!("x:float[4]; x.length", 4);
}

#[test]
fn test_typed_array_assignment_past_the_end_is_an_error() {
	fails_with("x : 3 int; x#5=7", "index out of range");
}
