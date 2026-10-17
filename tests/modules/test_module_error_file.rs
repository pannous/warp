// Card module-error-file: a failure in a module's code past parsing (an unknown word) names the module's file with its
// line, so it is not read as the program's line
use crate::common::fails_with;

#[test]
fn a_modules_unknown_word_names_its_file() {
	fails_with("use tests/fixtures/broken_word\nshout(\"hi\")", "tests/fixtures/broken_word.warp:4:");
}
