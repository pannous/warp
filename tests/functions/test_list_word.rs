// `list(x)` and `tuple(x)` call lib/prelude.warp's as_list (card sum-list): a program's own as_list stays apart
use warp::wasm_emitter::eval;

#[test]
fn list_word_calls_the_prelude() {
	assert_eq!(eval("as_list(x) := [x, x]; [as_list(1), list(\"ab\")]").serialize(), "[[1 1] ['a' 'b']]");
	assert_eq!(eval("tuple(\"ab\")").serialize(), "['a' 'b']");
}
