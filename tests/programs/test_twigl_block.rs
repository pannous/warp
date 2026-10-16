// cards twigl-make, twigl-float (user 2026-10-16): twigl code is a block like a shader block, not a text: `twigl { … }`
// is twigl("…") of lib/draw.warp, kept verbatim (warp format leaves it, the playground shows it as code), and a
// `$name` hole reads the warp variable name, a float, at each paint
#![cfg(feature = "native")]

/// What `warp eval` of the program says, stdout then stderr
fn said(program: &str) -> String {
	let run =
		crate::common::warp_command().args(["--no-ask", "eval", &format!("use graphics\n{program}")]).output().unwrap();
	format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr))
}

#[test]
fn a_twigl_block_is_the_twigl_call_of_its_code() {
	let lowered =
		warp::pipeline::lower("use graphics\ns = twigl {\n\to = vec4(1.);\n}").expect("a program").serialize();
	assert!(lowered.contains("o = vec4(1.);"), "{lowered}");
	let said = said("s = twigl {\n\to = vec4(1.);\n}\n\"fn warp_twigl\" in s");
	assert!(said.contains("yes"), "{said}");
}

#[test]
fn a_hole_in_a_twigl_block_reads_the_warp_variable() {
	// green is the hole's value: 1 in every pixel
	let program = "green = 1.0\ns = twigl {\n\to = vec4(0., $green, 0., 1.);\n}\npixels = gpu_render(s, 2, 2, {green: green})\npixels#1 == 0xFF00FF00";
	let said = said(program);
	if said.contains("no WebGPU adapter") {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert!(said.contains("yes"), "{said}");
}

#[test]
fn a_painted_twigl_block_gets_its_holes_values() {
	let lowered =
		warp::pipeline::lower("use graphics\ngreen = 1.0\ns = twigl { o = vec4(0., $green, 0., 1.); }\npaint(s, 2, 2)")
			.expect("a program")
			.serialize();
	assert!(lowered.contains("green:green") || lowered.contains("green: green"), "{lowered}");
}

// reviewer: only a dialect's call takes its block's holes; another function given a shader returns what it likes
#[test]
fn another_call_of_a_shader_takes_no_holes() {
	let lowered = warp::pipeline::lower("use graphics\nx = 1.0\nf(s) := s\ns = f(shader { $x })\npaint(s, 2, 2)")
		.expect("a program")
		.serialize();
	assert!(!lowered.contains("x:x") && !lowered.contains("x: x"), "{lowered}");
}
