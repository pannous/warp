// card twigl-glsl: twigl's backbuffer b, `texture(b, uv)`, is the shader's previous frame ($previous), uv from 0 to 1 with
// y up as GLSL counts (lib/draw.warp back_frame)
#![cfg(feature = "native")]

/// What `warp eval` of the program says, stdout then stderr
fn said(program: &str) -> String {
	let run =
		crate::common::warp_command().args(["--no-ask", "eval", &format!("use graphics\n{program}")]).output().unwrap();
	format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr))
}

#[test]
fn twigl_reads_its_previous_frame_through_b() {
	let adding = "s = twigl(\"o = texture(b, FC.xy / r) + vec4(.25, 0., 0., 1.);\")\nfirst = gpu_render(s, 2, 2)\nsecond = gpu_render(s, 2, 2)";
	// the bottom row blue each frame, the top row half of what it was: rows read upside down would make it blue too
	let rows = "u = twigl(\"o = vec4(0., 0., FC.y < 1. ? 1. : texture(b, FC.xy / r).b * .5, 1.);\")\ngpu_render(u, 1, 2)\nlast = gpu_render(u, 1, 2)";
	let said =
		said(&format!("{adding}\n{rows}\n[first#1 == 0xFF400000, second#1 == 0xFF800000, last#1 == 0xFF000000]"));
	if said.contains("no WebGPU adapter") {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert!(said.contains("[yes yes yes]"), "{said}");
}
