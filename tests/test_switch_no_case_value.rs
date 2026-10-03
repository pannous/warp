// #31 (user 2026-10-03, "Show runtime value"): a switch without a matching case names the subject's runtime value.
use crate::common::fails_with;

#[test]
fn no_case_shows_the_runtime_value() {
	fails_with("n=4; switch n {1: 2; 3: 4}", "no case for n = 4");
	fails_with("def f(n){ switch n {1: 2; 3: 4} }; f(5)", "no case for n = 5");
	fails_with("w=\"bc\"; switch w {\"a\": 1}", "no case for w = \"bc\"");
	fails_with("w='b'; switch w {'a': 1}", "no case for w = 'b'");
}
