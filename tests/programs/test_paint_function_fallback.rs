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
