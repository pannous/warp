// Node::add of two values it has no meaning for (a symbol and a number) is an Error naming both kinds, never a panic
// (card node-add; the meaning, an error or the list (a b), is pending with the user via warp-08)
use warp::Node;

#[test]
fn adding_unrelated_nodes_is_an_error_naming_both_kinds() {
	let sum = Node::Symbol("a".to_string()).add(Node::int(1));
	assert!(matches!(sum, Node::Error(_)), "{sum:?}");
	let message = sum.serialize();
	assert!(message.contains("symbol") && message.contains("int"), "{message}");
	assert_eq!(Node::int(1).add(Node::int(2)), Node::int(3));
}
