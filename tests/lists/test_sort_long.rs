//! Card runtime-speed: sorting a long list of Nodes (texts, objects by a key) takes n log n time, without recursion: a
//! bottom-up merge sort, stable, so items with equal keys keep their order
use crate::is;

/// Items sorted, and the milliseconds that may take: well under, quadratic takes seconds or exhausts the call stack
const ITEMS: i64 = 30_000;
const MILLISECONDS: i64 = 1500;

fn timed(fill: &str, sort: &str) -> String {
	format!("n = {ITEMS}; xs = []; for i in 0..n {{ xs.add({fill}) }}; t0 = clock(); q = {sort}; if clock() - t0 < {MILLISECONDS} then q[0] else -1")
}

#[test]
fn a_long_list_sorts_in_n_log_n() {
	is!(&timed("\"w\" + (n - i)", "sort(xs)"), "w1");
	is!(&timed("{k: n - i}", "xs.sort_by(p => p.k)").replace("q[0] else", "q[0].k else"), 1);
}

#[test]
fn equal_keys_keep_their_order() {
	is!("xs = [{k:2 v:1} {k:1 v:2} {k:2 v:3} {k:1 v:4}]; xs.sort_by(p => p.k).map(p => p.v)", warp::parse("[2 4 1 3]"));
	is!("sort([\"b\" \"a\" \"c\" \"a\"])", warp::parse("[\"a\" \"a\" \"b\" \"c\"]"));
	is!("sort(\"hello\")", "ehllo");
	is!("sort([])", warp::parse("[]"));
}

#[test]
fn ordered_input_sorts_in_linear_time() {
	is!(&timed("n - i", "sort(xs)"), 1);
	is!(&timed("i", "sort(xs)"), 0);
	// descending with equal keys is no strict reversal: the merge keeps their order
	is!("xs = [{k:2 v:1} {k:2 v:2} {k:1 v:3}]; xs.sort_by(p => p.k).map(p => p.v)", warp::parse("[3 1 2]"));
	is!("sort([5 4 3 2 1])", warp::parse("[1 2 3 4 5]"));
	is!("sort([1 2 2 3])", warp::parse("[1 2 2 3]"));
}
