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

// gpu_render's values: a map of numbers (f32) and lists of two to four numbers (vec2f…vec4f) the shader reads as
// `values.<name>` (a uniform), so a frame's values change without compiling the shader anew
const WITH_VALUES: &str = "shader = \"@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {
	return vec4f(values.red, values.tint.y, values.tint.z, 1.0);
}\"
gpu_render(shader, 1, 1, {red: 1.0, tint: [0.0, 1.0, 0.0]})";

#[test]
fn a_fragment_shader_reads_the_values_given() {
	let rendered = eval(WITH_VALUES);
	if matches!(&rendered, Node::Error(message) if message.to_string().contains("no WebGPU adapter")) {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert_eq!(rendered.serialize(), format!("[{}]", 0xFFFFFF00u32));
}

#[test]
fn a_value_that_is_no_number_says_so() {
	let failed = eval("gpu_render(\"@fragment fn main() -> @location(0) vec4f { return vec4f(1.0); }\", 1, 1, {name: \"x\"})");
	assert!(matches!(&failed, Node::Error(message) if message.to_string().contains("gpu_render") && message.to_string().contains("name")), "{failed:?}");
}

// card gpu-vectors (notes/gpu.md): over a linear float array the shader reads and writes the block in linear memory in
// place, no list built (~10 ns an item instead of ~1 µs); the name it is assigned to is the same array
#[test]
fn a_compute_shader_runs_over_a_linear_array_in_place() {
	let program = DOUBLING.replace("gpu_compute(shader, [1.5, 2.25, 3.0], 1)",
		"linear xs = float[3]\nxs#1 = 1.5; xs#2 = 2.25; xs#3 = 3\nys = gpu_compute(shader, xs, 1)\n[xs#1, ys#2, xs#3, #ys]");
	let computed = eval(&program);
	if matches!(&computed, Node::Error(message) if message.to_string().contains("no WebGPU adapter")) {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert_eq!(computed.serialize(), "[3 4.5 6 3]");
}

// card gpu-vectors (P214: floats go to the GPU only where the program allows f32): `ys = xs.map(f) @gpu` of a pure
// numeric f over a linear float array runs f as a WGSL kernel in f32, xs left as it was; without an adapter the CPU
// maps it after one warning, so the values are the same either way
#[test]
fn a_gpu_map_runs_a_numeric_lambda_as_a_kernel() {
	let program = "linear xs = float[4]\nxs#1 = 1; xs#2 = 2; xs#3 = 4; xs#4 = 9\nys = xs.map(x => x * x - √x + 0.5) @gpu\n[ys#1, ys#3, ys#4, #ys, xs#4]";
	assert_eq!(eval(program).serialize(), "[0.5 14.5 78.5 4 9]");
	assert_eq!(eval(&program.replace(") @gpu", ")").replace("ys = xs", "ys = @gpu xs")).serialize(), "[0.5 14.5 78.5 4 9]");
}

// P214 (user: "for @gpu give a warning or hint if the GPU does not apply"): a lambda WGSL cannot run, or a list not in
// linear memory, maps on the CPU with a warning saying why (an error under strict)
#[test]
fn a_gpu_map_the_gpu_cannot_run_says_why() {
	use warp::diagnostic::{with_warning_mode, WarningMode};
	with_warning_mode(WarningMode::Error, || crate::common::fails_with("linear xs = float[2]\nys = xs.map(x => str(x)) @gpu", "@gpu"));
	with_warning_mode(WarningMode::Error, || crate::common::fails_with("xs = [1.5, 2.5]\nys = xs.map(x => x * 2) @gpu", "@gpu"));
	assert_eq!(eval("xs = [1.5, 2.5]\nys = xs.map(x => x * 2) @gpu\nys").serialize(), "[3 5]");
}

// card gpu-threshold (notes/gpu.md, P214: err on the CPU side): an @gpu map of fewer than GPU_MAP_MIN_COUNT items runs
// on the CPU in f64 (the GPU's fixed ~2 ms loses below ~3·10^4 items), above it in f32 within f32's precision; a
// lambda the CPU's f64x2 kernel computes (~1 ns an item) never goes to the GPU, a warning says so
#[test]
fn a_gpu_map_runs_on_the_cpu_where_that_is_faster() {
	let tolerance = crate::common::gpu_tolerance();
	use warp::diagnostic::{with_warning_mode, WarningMode};
	let filled = |n: usize| format!("linear xs = float[{n}]\nfor i in 1 to {n} {{ xs#i = i / 30000.0 }}\nys = xs.map(x => sin(x)) @gpu\n");
	assert_eq!(eval(&format!("{}ys#1 == sin(xs#1)", filled(100))).serialize(), "yes");
	assert_eq!(eval(&format!("{}abs(ys#40000 - sin(xs#40000)) < {tolerance}", filled(40000))).serialize(), "yes");
	with_warning_mode(WarningMode::Error, || crate::common::fails_with("linear xs = float[2]\nys = xs.map(x => x * 2 + 1) @gpu", "f64x2"));
}

// card gpu-vectors: a @gpu lambda may read numbers of the program (`k`, `shift`): the kernel gets their values with
// each map; a lambda of only arithmetic stays on the CPU, with outer numbers too
#[test]
fn a_gpu_map_reads_outer_numbers() {
	use warp::diagnostic::{with_warning_mode, WarningMode};
	let program = "linear xs = float[40000]\nfor i in 1 to 40000 { xs#i = i / 40000.0 }\nk = 0.5; shift = 2\nys = xs.map(x => sin(x) * k + shift) @gpu\n";
	assert_eq!(eval(&format!("{program}abs(ys#40000 - (sin(1) * 0.5 + 2)) < 0.00001")).serialize(), "yes");
	with_warning_mode(WarningMode::Error, || crate::common::fails_with("linear xs = float[2]\nk = 3.0\nys = xs.map(x => x * k) @gpu", "CPU maps"));
}

// card gpu-vectors (chains kept on the GPU): `xs.map(f).map(g) @gpu` is one kernel of g after f, the items cross to
// the GPU and back once
#[test]
fn a_chain_of_gpu_maps_is_one_kernel() {
	let tolerance = crate::common::gpu_tolerance();
	let program = "linear xs = float[40000]\nfor i in 1 to 40000 { xs#i = i / 40000.0 }\nys = xs.map(x => x * 2).map(y => sin(y) + y) @gpu\n";
	assert_eq!(eval(&format!("{program}abs(ys#40000 - (sin(2) + 2)) < {tolerance}")).serialize(), "yes");
	let lowered = warp::pipeline::lower(&format!("{program}ys#1")).expect("a program").serialize();
	assert_eq!(lowered.matches("gpu_map_linear").count(), 1, "{lowered}");
}

// card gpu-vectors: an @gpu map of a list of floats (not in linear memory) copies its items into a block once and maps
// that, so the list need not be declared `linear`
#[test]
fn a_gpu_map_takes_a_list_of_floats() {
	let tolerance = crate::common::gpu_tolerance();
	use warp::diagnostic::{with_warning_mode, WarningMode};
	let program = "xs = float[40000]\nfor i in 1 to 40000 { xs#i = i / 40000.0 }\nys = xs.map(x => sin(x)) @gpu\n";
	let checked = format!("{program}abs(ys#40000 - sin(1)) < {tolerance}");
	assert_eq!(with_warning_mode(WarningMode::Error, || eval(&checked)).serialize(), "yes");
	assert_eq!(eval("ys = [0.5, 1.5].map(x => sin(x)) @gpu\n[#ys, ys#2 == sin(1.5)]").serialize(), "[2 yes]");
}

// card gpu-vectors: an @gpu map result used only as the source of later @gpu maps never comes back from the GPU: its
// map is folded into theirs, one round trip; read on the CPU too, it is mapped on its own
#[test]
fn a_gpu_map_result_mapped_again_stays_on_the_gpu() {
	let tolerance = crate::common::gpu_tolerance();
	use warp::diagnostic::{with_warning_mode, WarningMode};
	let program = "linear xs = float[40000]\nfor i in 1 to 40000 { xs#i = i / 40000.0 }\nys = xs.map(x => sin(x)) @gpu\nzs = ys.map(y => exp(y) + 1) @gpu\n";
	let checked = format!("{program}abs(zs#40000 - (exp(sin(1)) + 1)) < {tolerance}");
	assert_eq!(with_warning_mode(WarningMode::Error, || eval(&checked)).serialize(), "yes");
	let round_trips = |program: &str| warp::pipeline::lower(program).expect("a program").serialize().matches("gpu_map_linear").count();
	assert_eq!(round_trips(&format!("{program}zs#1")), 1);
	assert_eq!(round_trips(&format!("{program}zs#1 + ys#1")), 2);
	// xs changed between the maps: ys keeps the items before the change
	assert_eq!(round_trips(&program.replace("zs = ", "xs#1 = 5.0\nzs = ")), 2);
	let light_after = program.replace("exp(y) + 1", "y * y + 1");
	assert_eq!(with_warning_mode(WarningMode::Error, || eval(&format!("{light_after}abs(zs#40000 - (sin(1) * sin(1) + 1)) < {tolerance}"))).serialize(), "yes");
}

// card gpu-vectors: sum, min and max of an @gpu map reduce on the GPU: each workgroup leaves one partial result, only
// those come back and the CPU combines them; below GPU_MAP_MIN_COUNT items, on the CPU in f64
#[test]
fn a_reduction_of_a_gpu_map_reads_back_partial_results() {
	let tolerance = crate::common::gpu_tolerance();
	use warp::diagnostic::{with_warning_mode, WarningMode};
	let filled = |n: usize| format!("linear xs = float[{n}]\nfor i in 1 to {n} {{ xs#i = i / {n}.0 }}\n");
	let program = filled(40000);
	let close = |reduction: &str, expected: &str| format!("{program}c = 0.0\nfor i in 1 to 40000 {{ c = c + sin(xs#i) }}\nr = {reduction}(xs.map(x => sin(x)) @gpu)\nabs(r - {expected}) < {tolerance} * ({expected} + 1)");
	for (reduction, expected) in [("sum", "c"), ("max", "sin(1)"), ("min", "sin(1 / 40000)")] {
		assert_eq!(with_warning_mode(WarningMode::Error, || eval(&close(reduction, expected))).serialize(), "yes", "{reduction}");
	}
	let lowered = warp::pipeline::lower(&format!("{program}s = sum(xs.map(x => sin(x)) @gpu)\ns")).expect("a program").serialize();
	assert!(lowered.contains("gpu_reduce_linear"), "{lowered}");
	assert_eq!(eval(&format!("{}s = sum(xs.map(x => sin(x)) @gpu)\ns == sin(0.5) + sin(1)", filled(2))).serialize(), "yes");
}

// a list of floats in GC memory is copied into a block once, as for a map
#[test]
fn a_reduction_of_a_gpu_map_takes_a_list_of_floats() {
	let tolerance = crate::common::gpu_tolerance();
	let program = "xs = float[40000]\nfor i in 1 to 40000 { xs#i = i / 40000.0 }\ns = max(xs.map(x => sin(x)) @gpu)\n";
	let lowered = warp::pipeline::lower(&format!("{program}s")).expect("a program").serialize();
	assert!(lowered.contains("gpu_reduce_linear"), "{lowered}");
	assert_eq!(eval(&format!("{program}abs(s - sin(1)) < {tolerance}")).serialize(), "yes");
}

// card gpu-vectors: an @gpu map result read in a text hole between two maps is read on the CPU, so it is not folded
// into the later map (it was: the hole read a block never filled, "index out of range")
#[test]
fn a_gpu_map_result_read_in_a_text_hole_is_not_folded() {
	let tolerance = crate::common::gpu_tolerance();
	let program = "linear xs = float[40000]\nfor i in 1 to 40000 { xs#i = i / 40000.0 }\nys = xs.map(x => sin(x)) @gpu\nprint \"\\(ys#1)\"\nzs = ys.map(y => exp(y) + 1) @gpu\n";
	assert_eq!(warp::pipeline::lower(program).expect("a program").serialize().matches("gpu_map_linear").count(), 2);
	crate::is!(&format!("{program}abs(zs#40000 - (exp(sin(1)) + 1)) < {tolerance}"), true);
}

// card gpu-vectors kept-buffer: an @gpu map result read on the CPU comes back, but its buffer also stays on the GPU, and
// a later @gpu map or reduction of it starts from there (flags KEEP_RESULT 1, SOURCE_KEPT 2 on the host word); not when
// a statement between writes it or hands it on
#[test]
fn a_gpu_map_result_read_on_the_cpu_is_kept_on_the_gpu_for_the_next_map() {
	let tolerance = crate::common::gpu_tolerance();
	use warp::diagnostic::{with_warning_mode, WarningMode};
	let keeping = |program: &str| {
		let mut flags = vec![];
		warp::pipeline::lower(program).expect("a program").visit(&mut |part| if let Node::List(items, _, _) = part {
			if items.first().is_some_and(|head| ["gpu_map_linear", "gpu_reduce_linear"].contains(&head.name().as_str())) {
				flags.push(items.last().expect("arguments").serialize());
			}
		});
		flags
	};
	let program = "linear xs = float[40000]\nfor i in 1 to 40000 { xs#i = i / 40000.0 }\nys = xs.map(x => sin(x)) @gpu\nprint \"\\(ys#1) of \\(#ys)\"\nzs = ys.map(y => exp(y) + 1) @gpu\ns = sum(ys.map(y => cos(y)) @gpu)\n";
	assert_eq!(keeping(program), ["1", "2", "2"]);
	let checked = format!("{program}abs(zs#40000 - (exp(sin(1)) + 1)) < {tolerance} and abs(s - sum(ys.map(y => cos(y)))) < 0.01");
	assert_eq!(with_warning_mode(WarningMode::Error, || eval(&checked)).serialize(), "yes");
	assert_eq!(keeping(&program.replace("zs = ", "ys#1 = 5.0\nzs = ")), ["0", "0", "0"]);
	assert_eq!(keeping(&program.replace("zs = ", "f(ys)\nzs = ")), ["0", "0", "0"]);
}

// card gpu-kept: a producer of only arithmetic maps on the CPU, so it leaves no buffer on the GPU; its consumer uploads
// (with SOURCE_KEPT it read a stale buffer an earlier run kept for the same block)
#[test]
fn a_gpu_map_of_a_cpu_mapped_result_uploads_it() {
	let mut flags = vec![];
	let program = "linear xs = float[40000]\nys = xs.map(x => x * 2) @gpu\nprint ys#1\nzs = ys.map(y => exp(y) + 1) @gpu\n";
	warp::pipeline::lower(program).expect("a program").visit(&mut |part| if let Node::List(items, _, _) = part {
		if items.first().is_some_and(|head| head.name() == "gpu_map_linear") {
			flags.push(items.last().expect("arguments").serialize());
		}
	});
	assert_eq!(flags, ["0"]);
}

// card demo-simple (a mesh for the shader): a list of lists in gpu_render's values is an array of vec4f, one item each
// (missing coordinates 0), and a list of more than four numbers fills vec4f after vec4f: `values.corners[1].y`
const WITH_ARRAYS: &str = "shader = \"@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {
	return vec4f(values.corners[1].y, values.corners[0].z, values.flat[1].x, 1.0);
}\"
gpu_render(shader, 1, 1, {corners: [[0, 0, 1], [0, 1, 0]], flat: [0, 0, 0, 0, 1]})";

#[test]
fn a_fragment_shader_reads_lists_of_vectors() {
	let rendered = eval(WITH_ARRAYS);
	if matches!(&rendered, Node::Error(message) if message.to_string().contains("no WebGPU adapter")) {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	assert_eq!(rendered.serialize(), format!("[{}]", 0xFFFFFFFFu32));
}
