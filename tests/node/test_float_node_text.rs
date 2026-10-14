//! A float node writes its shortest exact text everywhere (decision float-text), also as the operand of a key, which
//! Node's Display prints: `2.5e-6 as float`, not `0.0000025 as float` (card writes-node, found by self-serializer)
use warp::{Node, Number, Op};

#[test]
fn a_float_node_writes_its_shortest_text() {
	let tiny = Node::Number(Number::Float(2.5e-6));
	assert_eq!(tiny.to_string(), "2.5e-6");
	assert_eq!(tiny.serialize(), "2.5e-6");
	let cast = Node::Key(Box::new(tiny), Op::As, Box::new(Node::Symbol("float".into())));
	assert_eq!(cast.serialize(), "2.5e-6 as float");
	assert_eq!(Node::Number(Number::Float(0.1 + 0.2)).to_string(), "0.30000000000000004");
}
