// `xs.map(f) @parallel` (wiki/Purpose.md): f runs on every item as a task, all at once; the results keep their order
use crate::is;

#[test]
fn a_parallel_map_gives_the_results_in_order() {
	is!("twice(x) := x * 2; @parallel [1,2,3].map(twice)", warp::ints(vec![2, 4, 6]));
	is!("twice(x) := x * 2; xs = [4, 5]; ys = @parallel xs.map(twice); ys", warp::ints(vec![8, 10]));
}

#[test]
fn a_parallel_map_runs_its_items_at_once() {
	is!("slow(x) := { sleep(500); x }; started = clock(); ys = @parallel [1,2,3].map(slow); clock() - started < 1200", true);
}

#[test]
fn a_parallel_map_takes_a_function_value() {
	is!("inc = x => x + 1; @parallel [1,2].map(inc)", warp::ints(vec![2, 3]));
	is!("inc = x => x + 1; job = go { inc(3) }; await job", 4);
}

/// `go` before a map or a for loop splits its items into tasks (user, 2026-10-06: "a dual use for the go keyword")
#[test]
fn go_maps_in_parallel() {
	is!("twice(x) := x * 2; go [1,2,3].map(twice)", warp::ints(vec![2, 4, 6]));
	is!("ys = go [1,2,3].map(x => x + 1); ys", warp::ints(vec![2, 3, 4]));
	is!("twice(x) := x * 2; xs = []; for i in 1 to 20 { xs.add(i) }; ys = go xs.map(twice); [#ys, ys#1, ys#20]", warp::ints(vec![20, 2, 40]));
	is!("slow(x) := { sleep(500 ms); x }; started = clock(); ys = go [1,2,3,4].map(slow); clock() - started < 1200", true);
	is!("twice(x) := x * 2; ys = [1,2].map(twice) @parallel; ys", warp::ints(vec![2, 4]));
}

#[test]
fn go_for_runs_its_iterations_in_parallel() {
	is!("shared total = 0; go for x in [1,2,3,4,5] { total += x }; total", 15);
	is!("shared n = 0; started = clock(); go for x in [1,2,3] { sleep(500 ms); n += 1 }; [n, clock() - started < 1200]", warp::ints(vec![3, 1]));
}

#[test]
fn a_parallel_loop_updating_a_copy_warns() {
	use warp::diagnostic::{with_warning_mode, WarningMode};
	with_warning_mode(WarningMode::Error, || crate::common::fails_with("t = 0; go for x in [1,2] { t += x }; t", "shared t"));
}

/// `@parallel square all xs` maps like `@parallel xs.map(square)` (card parallel-square)
#[test]
fn a_parallel_all_call_runs_its_items_at_once() {
	is!("square(x) := x * x; xs = [1, 2, 3]; @parallel square all xs", warp::ints(vec![1, 4, 9]));
	is!("square(x) := x * x; ys = @parallel square all [1, 2, 3]; ys", warp::ints(vec![1, 4, 9]));
	is!("slow(x) := { sleep(500 ms); x }; xs = [1, 2, 3]; started = clock(); ys = @parallel slow all xs; clock() - started < 1200", true);
}

/// `@parallel` on a form it can't split into tasks says so instead of running it sequentially without a word
#[test]
fn a_parallel_form_that_runs_sequentially_warns() {
	use warp::diagnostic::{with_warning_mode, WarningMode};
	with_warning_mode(WarningMode::Error, || crate::common::fails_with("square(x) := x * x; xs = [1, 2, 3]; @parallel square xs", "@parallel"));
}
