use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn a_block_using_it_is_a_function() {
	is!("f={it*2}; f 3", 6);
	is!("f={it*2}; f(4)", 8);
	is!("f:={it*2}; f 3", 6);
}

#[test]
fn data_blocks_stay_data() {
	assert_eq!(eval("person={name:\"x\"}; person").serialize(), "name:'x'");
	assert_eq!(eval("b={1 2}; b").serialize(), "{1 2}");
	assert_eq!(eval("c={a:1 b:2}; c").serialize(), "{a:1 b:2}");
}
