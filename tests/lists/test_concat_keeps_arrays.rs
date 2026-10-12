//! Card runtime-speed: `xs + ys` copies both lists (list_concat), so it only reads them: a list that is also
//! concatenated somewhere stays an array, and its index writes take constant time instead of walking cons cells
use crate::is;

/// Index writes, and the milliseconds they may take: a few at constant time, seconds when each write walks the list
const ITEMS: i64 = 50_000;
const MILLISECONDS: i64 = 2000;

fn timed(concatenation: &str) -> String {
	format!("xs = []; for i in 0..{ITEMS} {{ xs.push(i) }}; t0 = clock(); for i in 0..{ITEMS} {{ xs[i] = i }}; t1 = clock(); c = {concatenation}; if t1 - t0 < {MILLISECONDS} then #c else -1")
}

#[test]
fn index_writes_of_a_concatenated_list_take_constant_time() {
	is!(&timed("xs + [1]"), ITEMS + 1);
	is!(&timed("[1] + xs"), ITEMS + 1);
	is!(&timed("xs + xs"), 2 * ITEMS);
}

#[test]
fn a_concatenation_is_a_copy_of_both_lists() {
	is!("xs = [1 2]; ys = [3]; c = xs + ys; xs[0] = 9; ys.add(4); c", warp::parse("[1 2 3]"));
	is!("xs = [1 2]; c = xs + xs; xs.add(5); [#c, #xs]", warp::parse("[4 3]"));
}
