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
