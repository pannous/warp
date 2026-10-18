//! Card insert-at-warp: `xs.insert(v, at: i)` is lib/prelude.warp's `inserted(xs, i, v)` taken in place by list_become,
//! so every holder of the list sees the item
use crate::is;
use warp::ints;
use warp::node::strings;

#[test]
fn inserted_is_the_list_with_the_item_at_the_position() {
	is!("inserted([1, 2, 3], 1, 9)", ints(vec![1, 9, 2, 3]));
	is!("inserted([1, 2], 5, 9)", ints(vec![1, 2, 9]));
	is!("inserted([1, 2], -1, 9)", ints(vec![1, 2, 9]));
	is!("inserted([], 0, 5)", ints(vec![5]));
}

#[test]
fn insert_is_seen_by_every_holder() {
	is!("xs = [1, 2]; ys = xs; ys.insert(9, at: 1); xs", ints(vec![1, 9, 2]));
	is!("xs = [\"a\", \"b\"]; ys = xs; ys.insert(0, \"z\"); xs", strings(vec!["z", "a", "b"]));
}
