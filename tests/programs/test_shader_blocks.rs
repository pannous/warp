// card g_oFJc: WGSL as a sublanguage of warp, `shader { … }` (alias `wgsl { … }`): the braces' content verbatim, its
// value the shader text paint and gpu_render take, without quotes or escapes
use crate::is;

const FRAGMENT: &str = "@fragment fn main() -> @location(0) vec4f { return vec4f(1.0, 0.0, 0.0, 1.0); }";

#[test]
fn a_shader_block_is_its_wgsl_text() {
	is!(&format!("s = shader {{ {FRAGMENT} }}\ns"), FRAGMENT);
	is!(&format!("wgsl {{ {FRAGMENT} }}"), FRAGMENT);
}

#[test]
fn a_shader_block_over_lines_loses_its_indentation() {
	let code = "s = shader {\n\t@fragment fn main() -> @location(0) vec4f {\n\t\treturn vec4f(1.0); // a { in a comment\n\t}\n}\ns";
	is!(code, "@fragment fn main() -> @location(0) vec4f {\n\treturn vec4f(1.0); // a { in a comment\n}");
}

#[test]
fn shader_stays_a_name() {
	is!("shader = \"@fragment\"\nshader", "@fragment");
}

#[cfg(feature = "native")]
#[test]
fn paint_takes_a_shader_block() {
	use super::test_paint_lists::painted_rows;
	let probe = crate::common::warp_command().args(["--no-ask", "eval", &format!("gpu_render(\"{FRAGMENT}\", 1, 1)")]).output().unwrap();
	if format!("{probe:?}").contains("no WebGPU adapter") {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	let quoted = painted_rows(&format!("paint(\"{FRAGMENT}\", 2, 1)"), "paint-quoted-shader");
	assert_eq!(painted_rows(&format!("paint(shader {{ {FRAGMENT} }}, 2, 1)"), "paint-shader-block"), quoted);
}
