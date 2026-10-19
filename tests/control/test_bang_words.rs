// card warp-is-tests (decision warp-is-tests): a name ending in `!` is a word of its own once defined (`twice!`), and
// the prelude's `is! "3+3" 6` checks code against its value; under `warp test` each is! line counts as a test
use crate::is;
use warp::node::Node;
use warp::pipeline::{eval, for_tests};

#[test]
fn a_defined_bang_name_is_one_word() {
	is!("def twice!(x: int) := x * 2; twice! 4", 8);
	is!("twice!(x: int) := x * 2; twice!(5) + 1", 11);
	is!("y = 3; y!=4", true);
}

#[test]
fn is_bang_compares_the_value_of_code() {
	is!("is! \"3+3\" 6", true);
	is!("is! \"3+3\" 7", false);
	is!("is!(\"[1,2]\", [1,2])", true);
	is!("n = 1; n += 1; is! \"2*3\" 6", true);
	is!("is! \"3==2+1\" true", true);
}

#[test]
fn under_test_each_is_bang_counts() {
	assert_eq!(for_tests(|| eval("is! \"3+3\" 6\nis! \"'a'+'b'\" \"ab\"")), Node::Text("✓ 2 tests passed".into()));
	// code the program can't hold (π crossing to the block) fails its line only
	match for_tests(|| eval("is! \"3+3\" 6\nis! \"3+3\" 7\nis! \"1/\" 2\nis! \"π\" pi")) {
		Node::Error(message) => assert!(message.serialize().contains("3 of 4 failed"), "{message:?}"),
		other => panic!("expected the Error 3 of 4 failed, got {other:?}"),
	}
}
