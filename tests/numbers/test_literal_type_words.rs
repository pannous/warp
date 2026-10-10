//! `0.1:float`, `3:long`, `0.5:rational`: a number literal takes every word of a number type in BUILTIN_TYPES, aliases
//! too (card cleanup-closed-lists); the parser's own list missed long, rational, float32 and float64
use crate::is;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn every_number_type_word_types_a_literal() {
	assert_eq!(shown("type(3:long)"), "int");
	assert_eq!(shown("type(0.5:rational)"), "rational");
	assert_eq!(shown("(0.5:rational) + 1/3"), "5/6");
	is!("2:float64 / 4", 0.5);
	is!("1.5:int", 1);
	is!("7:i32 + 1", 8);
}
