// A list of Nodes built by appending (`out.add(x)`, what map and filter lower to) grows in place (card map-filter):
// it copied the whole list per item, quadratic, and 20000 texts exhausted the call stack
use crate::is;
use warp::*;

#[test]
fn map_and_filter_of_texts_stay_linear() {
	is!("count((1..100001).map(x => \"a\"))", 100000);
	is!("count((1..100001).filter(x => x % 2 == 0).map(x => \"b\"))", 50000);
	is!("xs = (1..4).map(x => \"a\" + x); xs", texts(vec!["a1", "a2", "a3"]));
}

#[test]
fn an_appended_list_of_nodes_keeps_its_items() {
	is!("out = []; for i in 1 to 3 { out.add(\"x\"); out.add(i) }; out", list(vec![text("x"), int(1), text("x"), int(2), text("x"), int(3)]));
}
