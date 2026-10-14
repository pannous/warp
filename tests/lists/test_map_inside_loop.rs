//! Card map-inside: a map, filter or collecting loop inside a loop body grows its list in place, as at top level: the
//! list it is aliased to at its end is no longer held once the next round gives it a fresh list before appending
use crate::is;

/// Items mapped, and the milliseconds that may take: ~10 at linear time, seconds when each append copies the list
const ITEMS: i64 = 100_000;
const MILLISECONDS: i64 = 3000;

fn timed_in_loop(statement: &str) -> String {
	format!(
		"xs = int[{ITEMS}]; t0 = clock(); for round in 1 to 2 {{ {statement} }}; t1 = clock(); \
		 if t1 - t0 < {MILLISECONDS} then #d else -1"
	)
}

#[test]
fn map_and_filter_in_a_loop_body_take_linear_time() {
	is!(&timed_in_loop("d = xs.map(x => x * 2)"), ITEMS);
	is!(&timed_in_loop("d = xs.filter(x => x >= 0)"), ITEMS);
	is!(&timed_in_loop("out = []; for x in xs { out = out + [x + 1] }; d = out"), ITEMS);
}

/// Card map-predeclared: the result variable declared empty before the loop is a typed array too
#[test]
fn map_into_a_variable_declared_before_the_loop_takes_linear_time() {
	is!(&timed_in_loop("d = xs.map(x => x * 2)").replace("for round", "d = []; for round"), ITEMS);
	is!("d = []; for r in 1 to 3 { d = [r].map(x => x * 2) }; d", warp::parse("[6]"));
	is!("d = ø; e = []; for r in 1 to 3 { e = d; d = [r] }; [d, e]", warp::parse("[[3] [2]]"));
}

#[test]
fn an_alias_after_the_fresh_list_still_parts_on_append() {
	is!("old = []; for r in 1 to 3 { out = [9]; old = out; out = out + [r] }; [old, out]", warp::parse("[[9] [9 3]]"));
	is!(
		"out = []; old = []; for r in 1 to 3 { if r == 2 { out = [] }; old = out; out = out + [r] }; old",
		warp::parse("[2]")
	);
	is!(
		"xs = [1 2]; zs = []; for r in 1 to 2 { ys = xs.map(x => x * r); zs = zs + [ys] }; zs",
		warp::parse("[[1 2] [2 4]]")
	);
}
