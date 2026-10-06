// P51/P62 (user, 2026-10-05): an unknown word next to a value in code is a loud error, `(cube 3)` and `[cube 3]` too; the
// data contexts are a `data` prefix, the values of an object literal and data mode
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn test_an_unknown_word_with_a_value_is_an_error_in_code() {
	fails_with("cube 3", "undefined: cube");
	fails_with("(cube 3)", "undefined: cube");
	fails_with("[cube 3]", "undefined: cube");
	fails_with("y = (3 apples); y", "undefined: apples");
}

#[test]
fn test_data_contexts_keep_unknown_words() {
	assert_eq!(shown("data cube 3"), "cube 3"); // P63: `data`, not `quote`
	assert_eq!(shown("x = {shape: cube 3}; x.shape"), "cube 3");
	assert_eq!(warp::parse_data("cube 3").serialize().trim(), "cube 3");
}

#[test]
fn test_known_words_are_untouched() {
	assert_eq!(shown("cube(x):=x*x*x; cube 3"), "27");
	assert_eq!(shown("print 3"), "ø"); // print gives nothing (issue #18)
	assert_eq!(shown("3 km"), "3 km");
}
