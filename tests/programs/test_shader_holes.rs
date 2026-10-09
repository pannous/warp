// card shader-holes (P235b amended 2026-10-09): `$name` in a shader block is an explicit hole as in sql templates,
// the WGSL reads it as `values.name` and warp passes the variable in each paint's values map; bare names never capture
use crate::is;

const HOLES: &str = "size = 2.0\nblue = 1.0\nglow = shader {\n\t@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {\n\t\treturn vec4f(at.x / $size, 0.0, $blue, 1.0);\n\t}\n}\n";

#[test]
fn a_hole_is_read_from_the_values() {
	is!("s = shader { let x = $size * size; }\ns", "let x = values.size * size;");
	is!("s = shader { let x = $1; }\ns", "let x = $1;");
}

#[test]
fn each_paint_passes_the_holes_given_no_value() {
	let lowered = warp::pipeline::lower(&format!("{HOLES}paint(glow, 2, 1)\npaint(glow, 2, 1, {{blue: 0.5}})")).expect("lowers").serialize();
	assert!(lowered.contains("(paint glow 2 1 {size:size, blue:blue})"), "{lowered}");
	assert!(lowered.contains("(paint glow 2 1 {blue:0.5, size:size})"), "{lowered}");
}

#[test]
fn values_not_written_out_cannot_take_the_holes() {
	let failure = warp::pipeline::lower(&format!("{HOLES}given = {{blue: 0.5}}\npaint(glow, 2, 1, given)")).expect_err("needs the map written out").serialize();
	assert!(failure.contains("size: size, blue: blue"), "{failure}");
}

#[cfg(feature = "native")]
#[test]
fn a_shader_with_holes_paints_as_with_the_values_given() {
	use super::test_paint_lists::painted_rows;
	let probe = crate::common::warp_command().args(["--no-ask", "eval", &format!("{HOLES}gpu_render(glow, 1, 1)")]).output().unwrap();
	if format!("{probe:?}").contains("no WebGPU adapter") {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	let given = painted_rows(&format!("{HOLES}paint(glow, 2, 1, {{size: 2.0, blue: 1.0}})"), "paint-holes-given");
	assert_eq!(painted_rows(&format!("{HOLES}paint(glow, 2, 1)"), "paint-holes"), given);
}
