// Card natural-phrases (samples/hello/natural.warp): the list phrases `keep only`, `sort by`, `take first` of a method chain,
// undoable defaults: `xs.keep only positive` is `xs where it > 0`, `xs.sort by size` sorts by the key, `xs.take first 2`
// is `xs.slice(0, 2)`
use crate::is;
use warp::warp_parser::parse;

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn keep_only_a_property() {
	is!("[3, -1, 2, -4].keep only positive", parse("[3 2]"));
	is!("[3, -1, 2, -4].keep only negative", parse("[-1 -4]"));
	is!("[1, 2, 3, 4].keep only even", parse("[2 4]"));
	is!("[1, 2, 3, 4].keep only odd", parse("[1 3]"));
	is!("small(n) := n < 3; [1, 2, 3, 4].keep only small", parse("[1 2]"));
}

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn sort_by_a_key() {
	is!("[\"bbb\", \"a\", \"cc\"].sort by size", parse("['a' \"cc\" \"bbb\"]"));
	is!("negated(n) := -n; [1, 3, 2].sort by negated", parse("[3 2 1]"));
	is!("items = [{price: 3}, {price: 1}]; cheapest = items.sort by price; cheapest#1.price", 1);
}

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn take_first_n() {
	is!("[1, 2, 3, 4].take first 2", parse("[1 2]"));
}

#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn a_chain_of_phrases() {
	is!("words = [\"ccc\", \"\", \"a\", \"bb\"]\nfull(t) := count(t) > 0\nresult = words\n    .keep only full\n    .sort by size\n    .take first 2\nresult", parse("['a' \"bb\"]"));
}

/// a parenthesized filter is closed: the method after it applies to the filtered list (it took the method into the
/// condition: "not a list")
#[test]
fn a_method_of_a_parenthesized_filter() {
	is!("w = [2, -1, 5]; (w where it > 0).slice(0, 1)", parse("[2]"));
}

/// card todo-app robustness: `todos sorted by priority` as written without a dot, and `sorted` as `sort`
#[ignore = "cleanup-edge: cleanup-park-phrases: phrase forms parked until card prepositions-call"]
#[test]
fn sorted_by_a_field_without_a_dot() {
	is!("items = [{price: 3}, {price: 1}]; cheapest = items sorted by price; cheapest#1.price", 1);
	is!("class T{p:int}; xs = [T(2), T(1)]; (xs sorted by p)#1.p", 1);
	is!("class T{p:int}; xs = [T(2), T(1)]; ys = xs.sorted by p; ys#1.p", 1);
	is!("xs = [1, -2, 3]; xs keep only positive", parse("[1 3]"));
}
