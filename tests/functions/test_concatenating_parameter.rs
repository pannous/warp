//! A parameter concatenated with a list literal (`ys + [x]`) is a list: the function takes it whole, no broadcasting
use crate::is;

#[test]
fn a_concatenated_parameter_takes_the_list() {
	is!("put(ys, x) := ys + [x]; put([1, 2], 3)", warp::parse("[1 2 3]"));
	is!("put(ys) := ys + [5]; put([1])", warp::parse("[1 5]"));
	is!("put(ys, x) := ys + [x]; out = []; put(out, \"ab\")", warp::parse("[\"ab\"]"));
	is!("sq(x) := x*x; sq([1, 2])", warp::parse("[1 4]"));
}
