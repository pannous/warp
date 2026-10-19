//! card shader-blocks-code: a comment or a text naming `shader {` is no shader block, so a shader error is still
//! placed in the program's one real block
#![cfg(feature = "native")]

const REAL_BLOCK: &str = "s = shader {\n\t\tlet b = c\n\t}";

/// What rendering the program's shader `s` says, None without a WebGPU adapter
fn rendering_says(program: &str) -> Option<String> {
	let said = crate::common::said(&format!("{program}\ngpu_render(s, 1, 1)"));
	(!said.contains("no WebGPU adapter")).then_some(said)
}

fn assert_placed(program: &str, place: &str) {
	let Some(said) = rendering_says(program) else {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	};
	assert!(said.contains(place), "{said}");
}

#[test]
fn the_real_block_alone_places_the_error() {
	assert_placed(REAL_BLOCK, "at 2:3");
}

#[test]
fn a_comment_naming_a_shader_block_is_no_block() {
	assert_placed(&format!("// shader {{ let b = c }}\n{REAL_BLOCK}"), "at 3:3");
}

#[test]
fn a_text_naming_a_shader_block_is_no_block() {
	assert_placed(&format!("note = \"shader {{ let b = c }}\"\n{REAL_BLOCK}"), "at 3:3");
}

#[test]
fn a_shader_error_after_a_hole_counts_the_hole_as_written() {
	assert_placed("x = 2\ns = shader {\n\tfn f() -> f32 { let a = $x; return c; }\n}", "at 3:37");
}
