//! Card recursive-list: the runtime functions that copy the cells of a list of Nodes (list_concat, list_insert_at) do it
//! iteratively, so a long list never exhausts the call stack
use crate::is;

/// 100001 texts: the last one empty
const LONG: &str = "xs = (100000 times \"a,\").split(\",\"); ";

#[test]
fn a_long_list_of_nodes_grows_without_exhausting_the_stack() {
	is!(&format!("{LONG}xs += [\"b\"]; [count(xs), xs#100002]"), warp::Node::List(vec![100002.into(), "b".into()], warp::node::Bracket::Square, warp::node::Separator::Space));
	is!(&format!("{LONG}ys = xs + [\"b\"]; count(ys)"), 100002);
	is!(&format!("{LONG}xs.insert(99999, \"b\"); [count(xs), xs#100000, xs#100001]"), warp::Node::List(vec![100002.into(), "b".into(), "a".into()], warp::node::Bracket::Square, warp::node::Separator::Space));
}

#[test]
fn copies_keep_the_order_and_share_nothing_they_change() {
	is!("xs = [\"a\", \"b\"]; ys = xs + [\"c\"]; [xs, ys]", warp::parse("[[\"a\" \"b\"] [\"a\" \"b\" \"c\"]]"));
	is!("xs = [\"a\", \"c\"]; xs.insert(1, \"b\"); xs", warp::parse("[\"a\" \"b\" \"c\"]"));
	is!("xs = [\"a\"]; xs.insert(0, \"z\"); xs.insert(-1, \"q\"); xs", warp::parse("[\"z\" \"a\" \"q\"]"));
	is!("xs = []; ys = xs + [\"a\"]; ys", warp::parse("[\"a\"]"));
}
