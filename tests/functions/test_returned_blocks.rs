// A block with `it` inside a function of one parameter: as a value (returned, assigned, passed) it is a function of its
// own `it`; elsewhere `it` is the function's parameter (`f(x) := x + it`)
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn a_returned_block_keeps_its_own_it() {
	is!("mk(k) := { return {it * k} }; f = mk(3); f(4)", 12);
	is!("def mk(k){ return {it * k} }; f = mk(3); f(4)", 12);
	is!("def mk(k){ {it + k} }; mk(10)(5)", 15);
}

#[test]
fn a_block_argument_keeps_its_own_it() {
	assert_eq!(eval("def scale(k){ map([1,2,3], {it * k}) }; scale(2)").serialize(), "[2 4 6]");
}

#[test]
fn it_in_the_body_is_the_parameter() {
	is!("f(x) := x + it; f(3)", 6);
	is!("def f(x){ it * 2 }; f(4)", 8);
}
