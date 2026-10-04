// A variable given values of two kinds (a list, then an Int) holds what it was given last: it is held as a Node and
// computes by the kind it has at run time (P45). It kept its first kind: `x = 5; x = [1]; x` was 1, a text then an Int
// a cast failure
use warp::*;

#[test]
fn a_variable_holds_its_last_value_of_another_kind() {
	is!("x = 5; x = [1]; x", ints(vec![1]));
	is!("x = [1,2]; x = 5; x + 1", 6);
	is!("x = \"a\"; x = 5; x + 1", 6);
}

#[test]
fn a_loop_variable_may_reuse_the_name_of_a_list() {
	is!("xs = [1 2]; s = 0; for xs in [5 6] { s += xs }; s", 11);
}
