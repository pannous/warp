// `list(x)` and `tuple(x)` call lib/prelude.warp's as_list (card sum-list): a program's own as_list stays apart
use warp::wasm_emitter::eval;

#[test]
fn list_word_calls_the_prelude() {
	assert_eq!(eval("as_list(x) := [x, x]; [as_list(1), list(\"ab\")]").serialize(), "[[1 1] ['a' 'b']]");
	assert_eq!(eval("tuple(\"ab\")").serialize(), "['a' 'b']");
}

/// a parameter keeps its own kind beside a global of its name that a closure captures (samples/events/arrow_keys.warp)
#[test]
fn a_parameter_is_not_the_global_of_its_name() {
	let program = "x = 160\ng(x) := x as list\non frame { x += 4 }\ng(\"ab\")";
	assert_eq!(eval(program).serialize(), "['a' 'b']");
}
