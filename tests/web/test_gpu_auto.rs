// card gpu-auto (user 2026-10-10: GPU maps switch on automatically above a size with an order of magnitude of room;
// GPU results are imprecise by design, @cpu keeps a computation exact): a heavy map of a linear float array runs on the
// GPU without @gpu from GPU_AUTO_MIN_COUNT items, with a run-time notice; below, of a light lambda or under @cpu, the CPU
// maps it in f64 (notes/gpu.md)
use warp::wasm_emitter::eval;

const AUTO_COUNT: usize = warp::gpu_maps::GPU_AUTO_MIN_COUNT as usize;

fn filled(count: usize) -> String {
	format!("linear xs = float[{count}]\nfor i in 1 to {count} {{ xs#i = i / {count}.0 }}\n")
}

/// The map ran on the CPU: its first item is exactly the f64 value `exact`, which f32 on the GPU is not
fn exact_on_cpu(program: &str, exact: &str) {
	assert_eq!(eval(&format!("{program}\nys#1 == {exact}")).serialize(), "yes", "{program}");
}

#[test]
fn a_heavy_map_of_many_floats_runs_on_the_gpu_by_itself() {
	let tolerance = crate::common::gpu_tolerance();
	let program = format!("{}ys = xs.map(x => sin(x))\n", filled(AUTO_COUNT));
	warp::errors::diagnostic::take_runtime_warnings();
	assert_eq!(eval(&format!("{program}abs(ys#{AUTO_COUNT} - sin(1)) < {tolerance}")).serialize(), "yes");
	#[cfg(feature = "native")]
	if warp::graphics::gpu::available().is_ok() {
		let notices = warp::errors::diagnostic::take_runtime_warnings();
		assert!(
			notices.iter().any(|notice| notice.contains("imprecise by design") && notice.contains("@cpu")),
			"{notices:?}"
		);
	}
}

#[test]
fn a_map_below_the_count_or_of_only_arithmetic_stays_on_the_cpu() {
	assert_eq!(eval(&format!("{}ys = xs.map(x => sin(x))\nys#1 == sin(xs#1)", filled(100))).serialize(), "yes");
	exact_on_cpu(&format!("{}ys = xs.map(x => x * 2 + 1)", filled(AUTO_COUNT)), "xs#1 * 2 + 1");
	// a list of numbers not in linear memory is not switched
	exact_on_cpu("ys = [0.5, 1.5].map(x => sin(x))", "sin(0.5)");
}

#[test]
fn cpu_pins_a_map_its_block_or_its_function_to_the_cpu() {
	let program = filled(AUTO_COUNT);
	exact_on_cpu(&format!("{program}ys = xs.map(x => sin(x)) @cpu"), "sin(xs#1)");
	exact_on_cpu(&format!("{program}ys = @cpu xs.map(x => sin(x))"), "sin(xs#1)");
	exact_on_cpu(&format!("{program}@cpu {{ ys = xs.map(x => sin(x)) }}"), "sin(xs#1)");
	// exact: the f64 values
	assert_eq!(eval(&format!("{program}ys = xs.map(x => sin(x)) @cpu\nys#{AUTO_COUNT} == sin(1)")).serialize(), "yes");
}

#[test]
fn gpu_and_cpu_on_one_map_is_an_error() {
	crate::common::fails_with(&format!("{}ys = xs.map(x => sin(x)) @gpu @cpu\nys#1", filled(4)), "@cpu");
}
