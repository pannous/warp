use warp::wasm_emitter::eval;

#[test]
fn test_square_brackets_keep_all_semicolon_items() {
	assert_eq!(eval("[1;2;3]"), eval("[1 2 3]"));
}

#[test]
fn test_square_brackets_keep_all_newline_items() {
	assert_eq!(eval("[1\n2\n3]"), eval("[1 2 3]"));
}
