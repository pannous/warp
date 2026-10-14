// card go-split-kernel: `go xs.map(x => …)` of a float array and a float kernel maps xs in linear memory, split above a
// count into slices that instances of their own map on separate threads (notes/simd.md); below it on one thread
use crate::is;
use warp::{float, int, list};

const FILL: &str = "xs = float[n]; for i in 1 to n { xs#i = i * 1.0 }\n";

fn lowered(code: &str) -> String {
	warp::pipeline::lower(code).expect("a program").serialize()
}

#[test]
fn a_go_map_of_a_float_kernel_splits_on_the_host() {
	let program = format!("n = 1000000 + random_below(1)\n{FILL}ys = go xs.map(x => x * 0.5 + 1)\n[ys#1, ys#n, #ys]");
	let code = lowered(&program);
	assert!(code.contains("task_split_mapf") && !code.contains("task_spawn_values"), "{code}");
	is!(&program, list(vec![float(1.5), float(500001.0), int(1000000)]));
}

#[test]
fn a_small_go_map_of_a_float_kernel_maps_on_one_thread() {
	is!(&format!("n = 5\n{FILL}ys = go xs.map(x => -sqrt(x) / 2)\n[ys#4, #ys]"), list(vec![float(-1.0), int(5)]));
}

#[test]
fn a_go_map_of_a_user_call_still_starts_tasks() {
	let program = format!("f(x) := x + 1\nn = 4\n{FILL}ys = go xs.map(x => f(x) * 2)\n#ys");
	assert!(lowered(&program).contains("task_spawn_values"), "{}", lowered(&program));
	is!(&program, 4);
}
