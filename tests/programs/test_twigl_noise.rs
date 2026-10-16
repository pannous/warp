// card twigl-glsl: twigl's noise, fsnoise, fsnoiseDigits, snoise2D and snoise3D (lib/draw.warp twigl_noise), in a
// twigl shader that calls one of them, and only then
#![cfg(feature = "native")]

/// What `warp eval` of the program says, stdout then stderr
fn said(program: &str) -> String {
	let run =
		crate::common::warp_command().args(["--no-ask", "eval", &format!("use graphics\n{program}")]).output().unwrap();
	format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr))
}

#[test]
fn twigl_noise_varies_from_pixel_to_pixel() {
	let program = "s = twigl(\"o = vec4(snoise2D(FC.xy * 3.7) * .5 + .5, snoise3D(vec3(FC.xy * 2.3, 1.)) * .5 + .5, fsnoise(FC.xy), 1.);\")\npixels = gpu_render(s, 3, 1)\n[pixels#1 != pixels#2, pixels#2 != pixels#3]";
	let said = said(program);
	if said.contains("no WebGPU adapter") {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert!(said.contains("[yes yes]"), "{said}");
}

#[test]
fn a_twigl_shader_without_noise_gets_none() {
	let said = said("\"snoise2D\" in twigl(\"o = vec4(1.);\")");
	assert!(said.contains("no"), "{said}");
}

// reviewer: a pasted shader bringing its own noise keeps it, no redefinition (twigl's code is a function body, so a
// shadertoy one), and shadertoy gets the noise as twigl does
#[test]
fn pasted_noise_stays_and_shadertoy_gets_noise_too() {
	let own = "s = twigl(\"o = vec4(fsnoise(FC.xy));\")\nt = twigl(\"o = vec4(1.);\")";
	let pasted = "shadertoy(\"float fsnoise(vec2 c) { return 0.5; }\\nvoid mainImage(out vec4 fragColor, in vec2 fragCoord) { fragColor = vec4(fsnoise(fragCoord)); }\")";
	let toy = "shadertoy(\"void mainImage(out vec4 fragColor, in vec2 fragCoord) { fragColor = vec4(snoise2D(fragCoord)); }\")";
	let said = said(&format!(
		"{own}\np = {pasted}\nq = {toy}\n[count(s) > count(t), \"fn snoise2D\" in p, \"fn snoise2D\" in q]"
	));
	assert!(said.contains("[yes no yes]"), "{said}");
}
