//! Card map-typed: a list aliased only after its appends (`d = (out = ø; for … { out = out + [x] }; out)`, every lowered
//! map and filter) grows in place, so map and filter of a typed list take linear time
use crate::is;

/// Items mapped, and the milliseconds that may take: ~10 at linear time, minutes when each append copies the list
const ITEMS: i64 = 100_000;
const MILLISECONDS: i64 = 3000;

fn timed(statement: &str) -> String {
	format!("xs = int[{ITEMS}]; t0 = clock(); {statement}; t1 = clock(); if t1 - t0 < {MILLISECONDS} then #d else -1")
}

#[test]
fn map_and_filter_of_a_typed_list_take_linear_time() {
	is!(&timed("d = xs.map(x => x * 2)"), ITEMS);
	is!(&timed("d = xs.filter(x => x >= 0)"), ITEMS);
	is!(&timed("out = []; for x in xs { out = out + [x + 1] }; d = out"), ITEMS);
}

#[test]
fn an_append_while_the_list_is_aliased_still_makes_a_new_list() {
	is!("xs = [1]; ys = xs; xs = xs + [2]; [#xs, #ys]", warp::parse("[2 1]"));
	is!("out = []; d = []; for i in 1 to 3 { d = out; out = out + [i] }; [#d, #out]", warp::parse("[2 3]"));
	is!("out = []; d = out; for i in 1 to 3 { out = out + [i] }; [#d, #out]", warp::parse("[0 3]"));
}
