// card create-little (user): twigl(code) runs a shader written in the shorthand of twigl.app's geekest mode: GLSL
// drawing the color o of the pixel FC (y up) from r, t, m and f, turned into WGSL by naga (crates/warp-glsl) and
// wrapped in a WGSL main (lib/draw.warp), so paint and gpu_render take it as any shader
#![cfg(feature = "native")]

/// What `warp eval` of the program says, stdout then stderr
fn said(program: &str) -> String {
	let run =
		crate::common::warp_command().args(["--no-ask", "eval", &format!("use graphics\n{program}")]).output().unwrap();
	format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr))
}

#[test]
fn a_twigl_shader_paints_with_glsl_rows_counted_from_the_bottom() {
	// blue only in the bottom row: GLSL's FC.y grows upwards, the image's rows downwards
	let program = "s = twigl(\"o = vec4(1., 0., FC.y < 1. ? 1. : 0., 1.);\")\npixels = gpu_render(s, 2, 2)\n[pixels#1 == 0xFFFF0000, pixels#3 == 0xFFFF00FF]";
	let said = said(program);
	if said.contains("no WebGPU adapter") {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert!(said.contains("[yes yes]"), "{said}");
}

#[test]
fn the_tunnel_demo_compiles_to_wgsl() {
	let demo = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/twigl.warp")).unwrap();
	let code = demo.lines().find(|line| line.starts_with("tunnel = twigl(")).expect("the demo's twigl line");
	let said = said(&format!("{code}\n\"fn warp_twigl\" in tunnel and \"@fragment fn main\" in tunnel"));
	assert!(said.contains("yes"), "{said}");
}

#[test]
fn a_glsl_mistake_shows_the_line() {
	let said = said("twigl(\"o = nonsense;\")");
	assert!(said.contains("Unknown variable: nonsense") && said.contains("o = nonsense;"), "{said}");
}

// the reviewer's sibling: shadertoy(code) takes mainImage(out vec4 fragColor, in vec2 fragCoord) with iResolution, iTime,
// iMouse and iFrame through the same GLSL wrapper (lib/draw.warp glsl_shader)
#[test]
fn a_shadertoy_shader_paints_through_the_same_wrapper() {
	let program = "s = shadertoy(\"void mainImage(out vec4 fragColor, in vec2 fragCoord) { fragColor = vec4(fragCoord.x < 1. ? 1. : 0., iResolution.x == 2. ? 1. : 0., fragCoord.y < 1. ? 1. : 0., 1.); }\")\npixels = gpu_render(s, 2, 2)\n[pixels#3 == 0xFFFFFFFF, pixels#2 == 0xFF00FF00]";
	let said = said(program);
	if said.contains("no WebGPU adapter") {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert!(said.contains("[yes yes]"), "{said}");
}
