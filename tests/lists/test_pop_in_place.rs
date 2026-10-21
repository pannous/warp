// Card shrink-in-place: `xs.pop()` drops the last item in place, copying nothing: a list drained by pops takes linear
// steps (each pop copied the whole list before), and every holder of the list still sees it shrink (P200b)
use crate::is;
use warp::util::with_fuel;
use warp::{ints, parse};

const DRAINED: &str = "def drained(n) { xs = []; for i in 1 to n { xs.add(i) }; total = 0; while #xs > 0 { total += xs.pop() }; total }\n";
/// 20000 pushes and pops: linear takes well under 10^7 steps, a copy per pop ~ 2·10^8
const LINEAR_BUDGET: u64 = 30_000_000;

#[test]
fn draining_a_list_by_pops_takes_linear_steps() {
	with_fuel(LINEAR_BUDGET, || is!(&format!("{DRAINED}drained(20000)"), 200010000));
}

#[test]
fn a_pop_gives_the_last_item_and_shrinks_the_list() {
	is!("xs = [1, 2, 3]; v = xs.pop(); [v, #xs]", ints(vec![3, 2]));
	is!("xs = [1, 2, 3]; ys = xs; xs.pop(); ys", ints(vec![1, 2]));
	is!("words = [\"a\", \"b\"]; w = words.pop(); [w, words]", parse("[\"b\" [\"a\"]]"));
	is!("s = {items: [1, 2]}; s.items.pop(); s.items", ints(vec![1]));
}

#[test]
fn the_last_pop_leaves_an_empty_list_that_grows_again() {
	is!("xs = [5]; xs.pop(); #xs", 0);
	is!("xs = [5]; xs.pop(); xs.add(7); xs", ints(vec![7]));
	is!("def f() { xs = [5]; xs.pop(); xs.add(7); xs.pop() + #xs }\nf()", 7);
}
