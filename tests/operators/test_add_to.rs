//! `add 4 to pixel` appends to the list, like pixel.add(4)

use crate::is;

#[test]
fn add_to_appends_an_element() {
	is!("pixel=[1 2 3]; add 4 to pixel; pixel", warp::ints(vec![1, 2, 3, 4]));
	is!("pixel=[1 2 3]; pixel.add(4); pixel", warp::ints(vec![1, 2, 3, 4]));
}

#[test]
fn add_to_takes_an_expression() {
	is!("pixel=[1 2]; n=2; add n+1 to pixel; pixel", warp::ints(vec![1, 2, 3]));
}

#[test]
fn add_to_an_empty_list() {
	is!("pixel=[]; add 4 to pixel; pixel", warp::ints(vec![4]));
}
