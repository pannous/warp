//! A zero-filled list of a large or computed size is built by a runtime loop, not expanded in the program

use warp::is;

#[test]
fn a_large_zero_list_has_its_size() {
	is!("pixels=640000*int; count(pixels)", 640000);
	is!("x : 100000 int; count(x)", 100000);
	is!("x : int[100000]; count(x)", 100000);
	is!("pixels=640000*int; pixels#640000", 0);
}

#[test]
fn a_computed_size_is_a_runtime_count() {
	is!("n=5; x = n*int; count(x)", 5);
	is!("w=800; h=800; pixels=w*h*int; count(pixels)", 640000);
}

#[test]
fn a_zero_list_is_still_a_list_of_zeros() {
	is!("x : 3 int; x", warp::ints(vec![0, 0, 0]));
	is!("x : 1000 int; x#3=4; x#3", 4);
	is!("x : 1000 int; x#3=4; x#2", 0);
}

#[test]
fn a_zero_list_of_nothing_is_empty() {
	is!("n=0; x = n*int; count(x)", 0);
}
