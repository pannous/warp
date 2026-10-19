use warp::node::{Bracket, Node, Separator};

fn list(bracket: Bracket) -> Node {
	Node::List(vec![Node::Symbol("a".into())], bracket, Separator::Semicolon)
}

#[test]
fn test_node_is_block() {
	assert!(list(Bracket::Curly).is_block());
	assert!(list(Bracket::Curly).with_comment("a block".into()).is_block());
	assert!(!list(Bracket::Square).is_block());
	assert!(!list(Bracket::Round).is_block());
	assert!(!Node::Symbol("a".into()).is_block());
}
