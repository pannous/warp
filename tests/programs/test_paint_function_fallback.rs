// card paint-function: paint(f, width, height) of a warp function goes to the GPU as f's fragment shader only where a
// shader can paint (the system value "gpu paints", asked as it paints); without a GPU (a browser without WebGPU, a
// machine without an adapter) f paints on the CPU, as a map falls back, a runtime warning saying so once
#![cfg(feature = "native")]
use warp_runtime::host_words::GPU_PAINTS;

const PROGRAM: &str = "width = 8\nheight = 4\nink(x, y) := (x + y) % 2\npaint(ink, width, height)";

#[test]
fn a_painted_function_has_its_cpu_painting_beside_its_shader() {
	let lowered = warp::pipeline::lower(PROGRAM).expect("a program").serialize();
	assert!(lowered.contains(GPU_PAINTS), "{lowered}");
	assert!(lowered.contains("@fragment"), "{lowered}");
	assert!(lowered.contains("(ink i%(width)"), "the CPU's painting: {lowered}");
}

#[test]
fn a_shader_paints_where_the_gpu_does() {
	let has_gpu = warp::graphics::gpu::available().is_ok();
	crate::is!("system_value(\"gpu paints\")", if has_gpu { 1 } else { 0 });
}

// reviewer: the GPU-or-CPU choice is an if around paint; a host word that only acts gives nothing (as print does), so a
// program ending in paint(f, w, h) shows no value, nor `if c { paint(…) }`, nor a function ending in paint
#[test]
fn a_painting_program_shows_no_value() {
	for code in [PROGRAM, "if 1 { paint([1, 0], 2, 1) }", "f() := paint([1, 0], 2, 1)\nf()"] {
		let run = crate::common::warp_command().args(["--no-ask", "eval", code]).output().unwrap();
		let shown = String::from_utf8_lossy(&run.stdout);
		assert!(
			run.status.success() && !shown.contains('»'),
			"{code}: {shown}{}",
			String::from_utf8_lossy(&run.stderr)
		);
	}
}
