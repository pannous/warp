// P234: paint also takes a WGSL fragment shader, renders it on the GPU and shows it, `paint(shader, w, h, values)`
// paints what `paint(gpu_render(shader, w, h, values), w, h)` paints; `use graphics` is `use draw`
#![cfg(feature = "native")]
use super::test_paint_lists::painted_rows;
use crate::is;

const SHADER: &str = "shader = \"@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {
	return select(vec4f(0.0, 0.0, values.blue, 1.0), vec4f(1.0, 0.0, 0.0, 1.0), at.x < 1.0);
}\"\n";

fn has_gpu() -> bool {
	let run = crate::common::warp_command().args(["--no-ask", "eval", &format!("{SHADER}gpu_render(shader, 1, 1, {{blue: 1.0}})")]).output().unwrap();
	!String::from_utf8_lossy(&run.stderr).contains("no WebGPU adapter") && !String::from_utf8_lossy(&run.stdout).contains("no WebGPU adapter")
}

#[test]
fn paint_renders_a_shader_text() {
	if !has_gpu() {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	let rendered = painted_rows(&format!("{SHADER}paint(gpu_render(shader, 2, 1, {{blue: 1.0}}), 2, 1)"), "paint-rendered");
	assert_eq!(painted_rows(&format!("{SHADER}paint(shader, 2, 1, {{blue: 1.0}})"), "paint-shader"), rendered);
}

#[test]
fn paint_of_a_shader_without_values_reads_none() {
	if !has_gpu() {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	let plain = "shader = \"@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f { return vec4f(1.0, 0.0, 0.0, 1.0); }\"\n";
	let rendered = painted_rows(&format!("{plain}paint(gpu_render(shader, 2, 1), 2, 1)"), "paint-plain-rendered");
	assert_eq!(painted_rows(&format!("{plain}paint(shader, 2, 1)"), "paint-plain-shader"), rendered);
}

#[test]
fn use_graphics_is_use_draw() {
	is!("use graphics\ncanvas(2, 1)\ndot(0, 0, red)\ncanvas_pixels[0] == red", true);
	is!("use graphics\nuse draw\ncanvas(1, 1)\nclear(blue)\ncanvas_pixels[0] == blue", true);
}
