// P47 stage 1: task variables inside a list literal are each awaited: `[a, b]` holds both results, not the last one
use warp::is;

#[test]
fn a_list_of_task_values_holds_each_result() {
	is!("f(x) := x*2; a = go f(1); b = go f(2); [a, b]", warp::ints(vec![2, 4]));
	is!("f(x) := x*2; a = go f(1); [a]", warp::ints(vec![2]));
	is!("f(x) := x*2; a = go f(1); b = go f(2); xs = [await a, await b]; count(xs)", 2);
}
