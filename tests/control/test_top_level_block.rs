use crate::is;
use warp::*;

#[test]
fn test_top_level_semicolons_yield_the_last_item() {
	is!("1;2;3", 3);
	is!("'hello';(1 2 3 4);10", 10);
}

#[test]
fn test_top_level_newlines_yield_the_last_item() {
	is!("1\n2\n3", 3);
}

#[test]
fn test_function_body_block_yields_its_last_item() {
	is!("f:={1;2;3};f()", 3);
}

#[test]
fn test_branch_block_yields_its_last_item() {
	is!("if true {1;2;3}", 3);
}

#[test]
fn test_data_object_block_keeps_all_keys() {
	assert_eq!(wasm_emitter::eval("{a:1\nb:2}").to_string(), "{a:1 b:2}");
}
