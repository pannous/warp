// gpu_compute over ints: a shader whose numbers are array<i32> (or array<u32>) gets them as 32-bit ints and gives back
// ints (notes/web_framework.md "web-apis: WebGPU")
use warp::wasm_emitter::eval;
use warp::{Node, Number};

const TRIPLING: &str = "shader = \"@group(0) @binding(0) var<storage, read_write> data: array<i32>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id: vec3<u32>) {
	if (id.x < arrayLength(&data)) { data[id.x] = data[id.x] * 3 + 1; }
}\"
gpu_compute(shader, [1, -2, 5], 1)";

#[test]
fn a_compute_shader_over_ints_gives_ints() {
	let computed = eval(TRIPLING);
	if matches!(&computed, Node::Error(message) if message.to_string().contains("no WebGPU adapter")) {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert_eq!(computed.serialize(), "[4 -5 16]");
	assert!(matches!(computed.first().drop_meta(), Node::Number(Number::Int(4))), "{computed:?}");
}
