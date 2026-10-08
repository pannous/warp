// card run-tests: tests run on demand (`warp test`, the playground's Run, pipeline::for_tests); every other build,
// optimized ones too, leaves them out of the module (P209)
use warp::pipeline::compile;

const TESTED: &str = "square(x) := x * x
test square(3) == 9
test \"squares\" {
  check square(4) == 16
}
square(5)";

const UNTESTED: &str = "square(x) := x * x
square(5)";

#[test]
fn a_build_leaves_the_tests_out() {
	let module = |code| compile(code).expect("a module").bytes;
	assert_eq!(module(TESTED), module(UNTESTED));
}
