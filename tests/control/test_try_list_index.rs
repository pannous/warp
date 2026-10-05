// Indexing a list inside `try`: errors are named runtime errors the `try` catches, never cast traps
use crate::common::fails_with;
use warp::is;

#[test]
fn indexing_empty_is_out_of_range() {
	fails_with("x=ø; x#1", "index out of range");
	fails_with("x=ø; x[0]", "index out of range");
	is!("x=ø; try x#1 else 7", 7);
	is!("def f(m){ m#1 }; try f(ø) else 7", 7);
}

// An element read as a number inside `try` (list_at) is not_an_int for a text, a list or ø, not a cast trap
#[test]
fn a_non_int_element_is_caught() {
	is!("x=[1,\"ab\"]; try -x#2 else 7", 7);
	is!("x=[1,[2]]; try x#2 % 2 else 7", 7);
	is!("x=[1,ø]; try x#2 + 1 else 7", 7);
	is!("x=[1,'a']; try -x#2 else 7", 7); // P65: a character is no number in arithmetic
	is!("x=[1,5]; try -x#2 else 7", -5);
}
