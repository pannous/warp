// card expect-make (user): `expect 30 10+20` is the soft test `test 10+20 == 30`: skipped by a plain run, counted
// by `warp test`; a program defining expect keeps its own word
use crate::is;
use warp::node::Node;
use warp::pipeline::{eval, for_tests};

#[test]
fn expect_is_a_soft_test() {
	is!("expect 30 10+20", Node::Empty);
	is!("expect 30 10+20; 7", 7);
	assert_eq!(for_tests(|| eval("expect 30 10+20")), Node::Text("✓ 1 test passed".into()));
	assert_eq!(for_tests(|| eval("square(x) := x * x; expect 9 square(3)")), Node::Text("✓ 1 test passed".into()));
	match for_tests(|| eval("expect 31 10+20; test 1 == 1")) {
		Node::Error(message) => assert!(message.serialize().contains("1 of 2 failed"), "{message:?}"),
		other => panic!("expected the Error 1 of 2 failed, got {other:?}"),
	}
}

#[test]
fn a_program_defining_expect_keeps_its_word() {
	is!("expect(a, b) := a * b; expect(3, 4)", 12);
}
