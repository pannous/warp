// cards map-library-first, map-library-word: a library word of one value before an iteration's list is its function,
// as a defined one is: map floor xs, map reverse xs, map count xs, map sorted xs
use crate::is;

#[test]
fn a_library_word_before_the_list_is_the_function() {
	is!("map floor [1.5 2.5]", warp::ints(vec![1, 2]));
	is!("xs = [[1 2] [3]]; map count xs", warp::ints(vec![2, 1]));
	is!("xs = [[1 2] [3]]; map(xs, count)", warp::ints(vec![2, 1]));
	is!("xs = [1.5]; map(floor, xs)", warp::ints(vec![1]));
	is!("ys = map reverse [[1 2] [3 4]]; ys#1#1", 2);
	is!("ys = map sorted [[2 1]]; ys#1#1", 1);
}

#[test]
fn a_defined_word_of_the_same_name_wins() {
	is!("floor(x) := x + 1; map floor [1]", warp::ints(vec![2]));
}
