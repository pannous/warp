// card test-soft (user, P209, P210): `test C` records whether C holds and goes on, `test "name" { … }` catches its
// failure per block; they run only under `warp test`, whose value is "✓ n tests passed" or the Error "m of n failed"
use crate::common::fails_with;
use crate::is;
use warp::node::Node;
use warp::pipeline::{eval, for_tests};

const PASSING: &str = "square(x) := x * x
test square(3) == 9
test \"squares\" {
  check square(4) == 16
  test square(0) == 0
}
square(5)";

const FAILING: &str = "square(x) := x * x
test square(3) == 10
test \"squares\" {
  check square(4) == 15
}
test \"fine\" { check square(1) == 1 }
test square(2) == 4
square(5)";

#[test]
fn a_plain_run_skips_the_tests() {
	is!(PASSING, 25);
	is!(FAILING, 25);
	is!("test 1 == 2", Node::Empty);
}

#[test]
fn under_test_the_value_is_the_summary() {
	assert_eq!(for_tests(|| eval(PASSING)), Node::Text("✓ 3 tests passed".into()));
	assert_eq!(for_tests(|| eval("test 1 + 1 == 2")), Node::Text("✓ 1 test passed".into()));
	match for_tests(|| eval(FAILING)) {
		Node::Error(message) => assert!(message.serialize().contains("2 of 4 failed"), "{message:?}"),
		other => panic!("expected the Error 2 of 4 failed, got {other:?}"),
	}
}

#[test]
fn a_program_naming_test_keeps_its_word() {
	is!("test(x) := x + 1; test 2", 3);
	fails_with("check 1 == 2; test 1 == 1", "assertion failed: 1 == 2");
}

#[cfg(feature = "native")]
#[test]
fn warp_test_prints_the_failures_and_exits_nonzero() {
	let directory = crate::common::scratch_directory("soft_tests");
	std::fs::create_dir_all(&directory).expect("a scratch directory");
	let run = |name: &str, program: &str| {
		let file = directory.join(name);
		std::fs::write(&file, program).expect("write the program");
		crate::common::warp_command().args(["test", &file.to_string_lossy()]).output().expect("warp runs")
	};
	let passed = run("passing.wasp", PASSING);
	assert_eq!(String::from_utf8_lossy(&passed.stdout).trim(), "✓ 3 tests passed");
	assert!(passed.status.success());
	let failed = run("failing.wasp", FAILING);
	let printed = String::from_utf8_lossy(&failed.stdout);
	assert!(printed.contains("✗ test square(3) == 10\n"), "{printed}");
	assert!(printed.contains("✗ \"squares\": assertion failed: square(4) == 15\n"), "{printed}");
	assert!(printed.trim_end().ends_with("2 of 4 failed"), "{printed}");
	assert_eq!(failed.status.code(), Some(1));
}
