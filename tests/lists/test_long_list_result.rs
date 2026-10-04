// A long list as the result of a program reads back whole: the cells are walked in a loop, not one stack frame each
// (100000 items overflowed the stack)
use warp::wasm_emitter::eval;
use warp::Node;

const LONG: usize = 100000;

#[test]
fn a_long_list_result_reads_back() {
	match eval(&format!("(0..{LONG}).map(x=>x)")) {
		Node::List(items, _, _) => assert_eq!(items.len(), LONG),
		other => panic!("not a list: {}", other.serialize().chars().take(200).collect::<String>()),
	}
}
