// Indexing ø (the empty list) is the named runtime error index_out_of_range, which a `try` catches, not a cast trap
use crate::common::fails_with;
use warp::is;

#[test]
fn indexing_empty_is_out_of_range() {
	fails_with("x=ø; x#1", "index out of range");
	fails_with("x=ø; x[0]", "index out of range");
	is!("x=ø; try x#1 else 7", 7);
	is!("def f(m){ m#1 }; try f(ø) else 7", 7);
}
