// card go-heavy-map: `go xs.map(x => sin(x))` of a linear float array and a heavy kernel takes the GPU switch of a plain
// map (card gpu-auto): the GPU from GPU_AUTO_MIN_COUNT items, else the CPU's loop; tasks copying slices of Nodes were
// fifty times slower than either (notes/gpu.md)
use warp::wasm_emitter::eval;

const AUTO_COUNT: usize = warp::gpu_maps::GPU_AUTO_MIN_COUNT as usize;

fn program(count: usize, last: &str) -> String {
	format!("n = {count} + random_below(1)\nxs = float[n]\nfor i in 1 to n {{ xs#i = i / n }}\nys = go xs.map(x => sin(x))\n{last}")
}

#[test]
fn a_go_map_of_a_heavy_kernel_takes_the_gpu_switch() {
	let lowered = warp::pipeline::lower(&program(AUTO_COUNT, "ys#n")).expect("a program").serialize();
	assert!(lowered.contains("gpu_map_linear") && !lowered.contains("task_spawn_values"), "{lowered}");
	let tolerance = crate::common::gpu_tolerance();
	assert_eq!(eval(&program(AUTO_COUNT, &format!("abs(ys#n - sin(1)) < {tolerance}"))).serialize(), "yes");
}

#[test]
fn a_small_go_map_of_a_heavy_kernel_is_exact_on_the_cpu() {
	assert_eq!(eval(&program(100, "[ys#100 == sin(1), #ys]")).serialize(), "[yes 100]");
}
