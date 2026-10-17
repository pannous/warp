// Card module-error-hidden: a module whose definition a source pass refuses (emit is a keyword) fails the program with
// that error at the module's file, not with "undefined function" at the program's use of a word the module defines
use crate::common::fails_with;

#[test]
fn a_modules_refused_definition_is_its_error() {
	fails_with("use tests/fixtures/keyword_definition\nok(3)", "emit is a keyword");
	fails_with("use tests/fixtures/keyword_definition\nok(3)", "tests/fixtures/keyword_definition.warp:2:");
}
