// decision warp-is-tests: a pairs file holds a test a line, code ⇥ expected ⇥ an optional note; `warp test` runs each
// as is!, and reports a commented-out pair as skipped with its note
use crate::is;
use warp::node::Node;
use warp::pipeline::{eval, for_tests};

const PAIRS: &str = "# arithmetic\n'1+1'\t2\n'0-1'\t-1\tnegative\n'x=0'\n\n'[1,2]'\t[1, 2]\n'3>1'\tyes\t";
const SKIPPING: &str = "'1+1'\t2\n// 'fib 10'\t55\tcard fib-memo: regression\n'2*3'\t6\n";

#[test]
fn each_pair_is_a_test() {
	assert_eq!(for_tests(|| eval(PAIRS)), Node::Text("✓ 5 tests passed".into()));
	match for_tests(|| eval("'1+1'\t3\n'2'\t2")) {
		Node::Error(message) => assert!(message.serialize().contains("1 of 2 failed"), "{message:?}"),
		other => panic!("expected the Error 1 of 2 failed, got {other:?}"),
	}
}

#[test]
fn a_tab_separates_the_columns_of_a_line() {
	is!("rows = ['a'\t1\n'y'\tø\tnote]; count(rows)", 2);
	is!("rows = ['a'\t1\n'y'\tø\t3]; rows#2#3", 3); // an empty column keeps its place
	is!("row = ['0-1'\t-1]; row#2", -1);
	is!("a = 1\t\t// a comment after tabs\na + 1", 2);
	is!("'y' ø\n'z'", 'z'); // a skipped ø at a line's end still ends its line (card line-ending)
}

#[cfg(feature = "native")]
#[test]
fn warp_test_reports_commented_pairs_as_skipped() {
	let directory = crate::common::scratch_directory("test_pairs");
	std::fs::create_dir_all(&directory).expect("a scratch directory");
	let file = directory.join("skipping.pairs.warp");
	std::fs::write(&file, SKIPPING).expect("write the pairs");
	let run = crate::common::warp_command().args(["test", &file.to_string_lossy()]).output().expect("warp runs");
	let printed = String::from_utf8_lossy(&run.stdout);
	assert!(printed.contains("⊘ skipped \"fib 10\" 55: card fib-memo: regression\n"), "{printed}");
	assert!(printed.trim_end().ends_with("✓ 2 tests passed"), "{printed}");
	assert!(run.status.success());
}
