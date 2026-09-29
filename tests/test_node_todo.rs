use warp::node::{error, Node};

#[test]
fn todo_is_a_loud_error_naming_the_missing_feature() {
	let placeholder = Node::todo("bignum division".to_string());
	assert_eq!(placeholder, error("not implemented yet: bignum division"));
	assert!(placeholder.first_error().is_some());
}
