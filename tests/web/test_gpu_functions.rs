// card gpu-functions (notes/gpu.md "Warp functions on the GPU"): a map's kernel may call the program's own functions,
// which become WGSL functions beside it: their local variables, if/else, for over a range, while and returns; a
// function WGSL cannot compute (a text, recursion) keeps the map on the CPU. Mass work goes to the GPU in f32 by itself
// as P240 says (user 2026-10-13), small maps stay exact
use warp::wasm_emitter::eval;

const AUTO_COUNT: usize = warp::gpu_maps::GPU_AUTO_MIN_COUNT as usize;
const FUNCTIONS: &str = "
shade(x) := sin(x) * 0.5 + 1
def damped(x) {
	level = 0.0
	for i in 0..4 {
		level = level + cos(x * i)
	}
	if level > 2 {
		return level / 2
	}
	level
}
def settled(x) {
	steps = 0
	y = x
	while y > 0.01 {
		y = y * 0.5
		steps = steps + 1
	}
	steps + shade(y)
}
";

fn filled(count: usize) -> String {
	format!("{FUNCTIONS}linear xs = float[{count}]\nfor i in 1 to {count} {{ xs#i = i / {count}.0 }}\n")
}

/// The last item of the map on the GPU, close to the same map pinned to the CPU
fn agrees_with_cpu(lambda: &str) {
	let tolerance = crate::common::gpu_tolerance();
	let program = format!("{}ys = xs.map({lambda})\nexact = xs.map({lambda}) @cpu\n", filled(AUTO_COUNT));
	let close = format!("{program}abs(ys#{AUTO_COUNT} - exact#{AUTO_COUNT}) + abs(ys#7 - exact#7) < {tolerance}");
	assert_eq!(eval(&close).serialize(), "yes", "{lambda}");
}

#[test]
fn a_map_calls_a_function_of_one_expression_on_the_gpu() {
	agrees_with_cpu("x => shade(x)");
	agrees_with_cpu("x => shade(x) * shade(x / 2)");
}

#[test]
fn a_map_calls_functions_of_statements_on_the_gpu() {
	agrees_with_cpu("x => damped(x)");
	agrees_with_cpu("x => settled(x)");
}

#[test]
fn a_function_wgsl_cannot_compute_keeps_the_map_on_the_cpu() {
	let program = filled(AUTO_COUNT);
	// exactly the f64 value, which f32 on the GPU is not
	let named = format!(
		"{program}def named(x) {{ if x > 0.5 {{ return count(\"ab\") }}; x }}\nys = xs.map(x => named(x))\nys#1 == named(xs#1)"
	);
	assert_eq!(eval(&named).serialize(), "yes");
	let recursive = format!(
		"{program}def halves(x) {{ if x < 0.1 {{ return x }}; halves(x / 2) }}\nys = xs.map(x => sin(halves(x)))\nys#1 == sin(halves(xs#1))"
	);
	assert_eq!(eval(&recursive).serialize(), "yes");
}

#[test]
fn a_small_map_calling_a_function_stays_exact() {
	assert_eq!(eval(&format!("{}ys = xs.map(x => shade(x))\nys#3 == shade(xs#3)", filled(100))).serialize(), "yes");
}
