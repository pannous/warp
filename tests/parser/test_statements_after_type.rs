//! A type declaration runs nothing: the statements around it still run

use crate::is;

#[test]
fn variables_assigned_next_to_a_type_declaration_keep_their_values() {
	is!("type point{x:int y:int}; q=5; q", 5);
	is!("q=5; type point{x:int y:int}; q", 5);
	is!("type point{x:int}; r=6; q=5; r+q", 11);
	is!("struct point{x:int}; q=[1 2]; q", warp::ints(vec![1, 2]));
}

#[test]
fn a_type_declaration_alone_is_empty() {
	is!("type point{x:int}; 4", 4);
}
