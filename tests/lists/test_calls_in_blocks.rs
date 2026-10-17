use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn calls_are_lowered_inside_single_statement_blocks() {
	is!("if yes {sum [1 2]}", 3);
	is!("if yes {sum 1+1 2+2}", 6);
	is!("if no {sum [1 2]} else {sum [4 5]}", 9);
	is!("if yes {max [4 5]}", 5);
	is!("if no {max 4 5} else {min 4 5}", 4);
}

// `x = {a b}` assigns the data `{a b}` (lowering/nodes.rs is_assigned_data), also when `a` is a library word
#[test]
fn an_assigned_block_stays_data() {
	for (code, data) in [("x = {sum [1 2]}; x", "{sum [1 2]}"), ("x = {max 3 4}; x", "{max 3 4}")] {
		assert_eq!(eval(code).serialize(), data, "{code}");
	}
}
