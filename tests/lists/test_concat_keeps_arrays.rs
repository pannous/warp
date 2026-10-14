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

/// Card runtime-speed step 4: two int (float) arrays concatenated into an array variable are copied with array.copy,
/// no Node made of an item: a million items ten times in well under the time boxing them took
const BULK_ITEMS: i64 = 1_000_000;
const BULK_MILLISECONDS: i64 = 300;

#[test]
fn concatenated_arrays_copy_in_bulk() {
	is!(&format!("xs = []; for i in 0..{BULK_ITEMS} {{ xs.push(i) }}; t0 = clock(); for k in 0..10 {{ c = xs + xs }}; if clock() - t0 < {BULK_MILLISECONDS} then c[{BULK_ITEMS} + 2] else -1"), 2);
	is!("xs = [1 2]; ys = [3]; c = xs + ys + [4 5] + xs; c[5] = 9; [c[4], c[5], #c, #xs]", warp::parse("[5 9 7 2]"));
	is!("xs = [1.5 2.5]; ys = []; ys.add(0.5); c = ys + xs; c[1] + c[2]", 4.0);
	is!("xs = [1 2]; e = []; e.add(3); e[0] = 7; c = e + xs; c", warp::parse("[7 1 2]"));
}
