// `print xs` of a list writes the text str(xs) gives, nested lists in brackets, and is worth the list (user, P32)
use crate::is;
use warp::*;

#[test]
fn print_of_a_list_is_worth_the_list() {
	is!("xs=[1,2]; y = print xs; count(y)", 2);
	is!("print [[1,2],[3]]", list(vec![ints(vec![1, 2]), ints(vec![3])]));
}

#[test]
#[cfg(feature = "native")]
fn print_of_a_list_writes_its_text() {
	assert!(crate::common::printed("xs=[1,2]; print xs").starts_with("[1 2]\n"));
	assert!(crate::common::printed("print [[1,2],[3]]").starts_with("[[1 2] [3]]\n"));
	assert!(crate::common::printed("for x in [[1],[2,3]] { print x }").starts_with("[1]\n[2 3]\n"));
}
