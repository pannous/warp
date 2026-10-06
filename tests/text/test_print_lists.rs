// `print xs` of a list writes the text str(xs) gives, nested lists in brackets (user, P32), and gives nothing (issue #18)
use crate::is;

#[test]
fn print_of_a_list_gives_nothing() {
	is!("xs=[1,2]; y = print xs; y", warp::Node::Empty);
	is!("print [[1,2],[3]]", warp::Node::Empty);
}

#[test]
#[cfg(feature = "native")]
fn print_of_a_list_writes_its_text() {
	assert!(crate::common::printed("xs=[1,2]; print xs").starts_with("[1 2]\n"));
	assert!(crate::common::printed("print [[1,2],[3]]").starts_with("[[1 2] [3]]\n"));
	assert!(crate::common::printed("for x in [[1],[2,3]] { print x }").starts_with("[1]\n[2 3]\n"));
}
