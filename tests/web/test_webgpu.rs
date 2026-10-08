// card web-apis (WebGPU): gpu_compute(shader, numbers, workgroups) runs a WGSL compute shader over the numbers in the
// browser (web/playground/host-gpu.js, on a task Worker) and natively (src/gpu.rs, wgpu) and gives back what it left
use warp::wasm_emitter::eval;
use warp::Node;

const DOUBLING: &str = "shader = \"@group(0) @binding(0) var<storage, read_write> data: array<f32>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id: vec3<u32>) {
	if (id.x < arrayLength(&data)) { data[id.x] = data[id.x] * 2.0; }
}\"
gpu_compute(shader, [1.5, 2.25, 3.0], 1)";

#[test]
fn a_compute_shader_runs_over_the_numbers() {
	let computed = eval(DOUBLING);
	if matches!(&computed, Node::Error(message) if message.to_string().contains("no WebGPU adapter")) {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	// floats, as the shader's f32 left them (also the whole ones)
	assert_eq!(computed.serialize(), "[3 4.5 6]");
}

#[test]
fn a_shader_that_does_not_compile_says_where() {
	let failed = eval("gpu_compute(\"fn main( {\", [1.0], 1)");
	assert!(matches!(&failed, Node::Error(message) if message.to_string().contains("gpu_compute")), "{failed:?}");
}

// card g_YqWY (WebGPU example): gpu_render(shader, width, height) runs a WGSL fragment shader `main` over every pixel
// and gives the pixels paint shows, 0xFFRRGGBB row by row
const HALVES: &str = "shader = \"@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {
	return select(vec4f(0.0, 0.0, 1.0, 1.0), vec4f(1.0, 0.0, 0.0, 1.0), at.x < 1.0);
}\"
gpu_render(shader, 2, 2)";

#[test]
fn a_fragment_shader_renders_the_pixels() {
	let rendered = eval(HALVES);
	if matches!(&rendered, Node::Error(message) if message.to_string().contains("no WebGPU adapter")) {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	let (red, blue) = (0xFFFF0000u32, 0xFF0000FFu32);
	assert_eq!(rendered.serialize(), format!("[{red} {blue} {red} {blue}]"));
}

#[test]
fn a_render_shader_that_does_not_compile_says_where() {
	let failed = eval("gpu_render(\"@fragment fn main( {\", 2, 2)");
	assert!(matches!(&failed, Node::Error(message) if message.to_string().contains("gpu_render") && message.to_string().contains("1:")), "{failed:?}");
}
