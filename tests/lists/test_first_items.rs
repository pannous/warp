// card first-first: `xs.first(3)`, `first(xs, 3)` and `first 3 of xs` are the first 3 items, `last 2 of xs` the last
// 2, fewer when the list is shorter (they were "first takes 1 argument" and "undefined: of"); `first of xs` stays one item
use crate::is;
use warp::*;

#[test]
fn first_and_last_take_a_number_of_items() {
	is!("xs = [5, 6, 7, 8]; xs.first(3)", ints![5, 6, 7]);
	is!("xs = [5, 6, 7, 8]; first(xs, 3)", ints![5, 6, 7]);
	is!("xs = [5, 6, 7, 8]; first 3 of xs", ints![5, 6, 7]);
	is!("xs = [5, 6, 7, 8]; last 2 of xs", ints![7, 8]);
	is!("xs = [5, 6]; xs.last(9)", ints![5, 6]);
	is!("\"hello\".first(2)", "he");
}

#[test]
fn first_of_a_list_is_its_first_item() {
	is!("xs = [5, 6, 7, 8]; first of xs", 5);
	is!("xs = [5, 6, 7, 8]; xs.last", 8);
}
