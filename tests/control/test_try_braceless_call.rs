// `try f x else y`: the guarded part may be a braceless call, it is no missing `else`
use crate::common::fails_with;
use crate::is;

#[test]
fn a_braceless_call_is_guarded() {
	// an unknown word applied to a value is still silent data (todo.md, branch unknown-word), but no longer a missing else
	let raised = warp::wasm_emitter::eval("try raise \"boom\" else 3").serialize();
	assert!(!raised.contains("needs an `else`"), "{raised}");
	is!("f(x) := x * 2; try f 4 else 3", 8);
	is!("xs = [1 2]; f(i) := xs#i; try f 5 else 3", 3);
	fails_with("try 1", "`try` needs an `else`");
}
