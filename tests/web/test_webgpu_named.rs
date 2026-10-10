// gpu_compute over named arrays (card web-apis-rest): a map {xs: […], ns: […]} binds each array where the shader declares
// the storage array of that name, and gives back the map of the arrays the shader left (notes/web_framework.md
// "web-apis: WebGPU")
use warp::wasm_emitter::eval;
use warp::Node;

const SUMS_AND_STEPS: &str = "shader = \"@group(0) @binding(0) var<storage, read_write> xs: array<f32>;
@group(0) @binding(1) var<storage, read_write> ys: array<f32>;
@group(0) @binding(2) var<storage, read_write> ns: array<i32>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id: vec3<u32>) {
	if (id.x < arrayLength(&xs)) { ys[id.x] = xs[id.x] + ys[id.x]; ns[id.x] = ns[id.x] - 1; }
}\"
gpu_compute(shader, {xs: [1.5 2 3], ys: [10 20 30], ns: [7 8 9]}, 1)";

const UNDECLARED: &str = "shader = \"@group(0) @binding(0) var<storage, read_write> xs: array<f32>;
@compute @workgroup_size(1) fn main() { xs[0] = 1.0; }\"
gpu_compute(shader, {xs: [0], zs: [0]}, 1)";

fn without_adapter(computed: &Node) -> bool {
	matches!(computed, Node::Error(message) if message.to_string().contains("no WebGPU adapter"))
}

#[test]
fn named_arrays_come_back_by_name() {
	let computed = eval(SUMS_AND_STEPS);
	if without_adapter(&computed) {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert_eq!(computed.serialize(), "{xs:[1.5 2 3] ys:[11.5 22 33] ns:[6 7 8]}");
}

#[test]
fn a_name_the_shader_does_not_declare_fails_loudly() {
	let computed = eval(UNDECLARED);
	if without_adapter(&computed) {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert!(computed.serialize().contains("declares no storage array zs"), "{computed:?}");
}

#[test]
fn the_sample_reads_an_array_by_name() {
	let computed = eval("samples/gpu_arrays.warp");
	if without_adapter(&computed) {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert_eq!(computed.serialize(), "[6 6 2]");
}
