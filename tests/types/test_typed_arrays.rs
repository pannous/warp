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

#[test]
fn a_type_word_subscript_is_a_zero_filled_list_anywhere() {
	is!("#(int[3])", 3);
	is!("n=3; #(int[n])", 3);
	is!("n=2; x = int[n]; x#2", 0);
}

#[test]
fn a_typed_list_read_before_its_assignment_is_empty() {
	is!("if 0 { x = int[3] }; x", Node::Empty);
}

#[test]
fn a_typed_list_beyond_an_array_index_is_out_of_memory() {
	fails_with("x = int[3000000000]; 1", "out of memory");
}
