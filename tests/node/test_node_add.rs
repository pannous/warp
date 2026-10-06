// Node::add of two values it has no number, text or list meaning for (a symbol and a number) builds the unevaluated
// sum `a + b`, never a panic (user, P86: lazily allowed; evaluated, both operands must be addable)
use warp::Node;

#[test]
fn adding_unrelated_nodes_builds_the_unevaluated_sum() {
	let sum = Node::Symbol("a".to_string()).add(Node::int(1));
	assert_eq!(sum.serialize(), "a+1");
	assert_eq!(Node::int(1).add(Node::int(2)), Node::int(3));
}
