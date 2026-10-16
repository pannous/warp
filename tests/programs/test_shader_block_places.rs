//! card shader-blocks-code: a comment or a text naming `shader {` is no shader block, so a shader error is still
//! placed in the program's one real block
use warp::graphics::shader_errors::place_in_program;

const REAL_BLOCK: &str = "s = shader {\n\t\tlet b = c\n\t}";

#[test]
fn the_real_block_alone_places_the_error() {
	assert_eq!(place_in_program(REAL_BLOCK, 1, 9), Some((2, 11)));
}

#[test]
fn a_comment_naming_a_shader_block_is_no_block() {
	let code = format!("// shader {{ let b = c }}\n{REAL_BLOCK}");
	assert_eq!(
		place_in_program(&code, 1, 9),
		place_in_program(REAL_BLOCK, 1, 9).map(|(line, column)| (line + 1, column))
	);
}

#[test]
fn a_text_naming_a_shader_block_is_no_block() {
	let code = format!("note = \"shader {{ let b = c }}\"\n{REAL_BLOCK}");
	assert_eq!(
		place_in_program(&code, 1, 9),
		place_in_program(REAL_BLOCK, 1, 9).map(|(line, column)| (line + 1, column))
	);
}
