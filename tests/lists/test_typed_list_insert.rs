//! Card w0-list-writes: `xs.insert(v, at: i)` is a write into the list like `xs += [v]`, so a declared element type
//! checks the inserted value; the position `at: i` is no item
use crate::common::fails_with;
use crate::is;

#[test]
fn insert_checks_declared_element_type() {
	is!("xs: [int] = [1]; xs.insert(3, at: 0); xs", warp::ints(vec![3, 1]));
	fails_with("xs: [int] = [1]; xs.insert(\"a\", at: 0)", "xs is declared list of int, cannot hold");
	fails_with("xs: [int] = [1]; v = \"a\"; xs.insert(v, at: 0)", "cannot hold an item that is no int");
}
