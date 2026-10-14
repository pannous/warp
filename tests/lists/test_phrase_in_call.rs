// card phrase-in-call: a list phrase as a call's argument `str(xs.take first 3)` (it was "take is in the standard
// module list"), and `xs.sort by it` sorting by an expression of `it` (it was "no field it"); found editing samples/orm.warp
use crate::is;
use warp::ints;

#[test]
fn a_list_phrase_as_the_argument_of_a_call() {
	is!("xs = [1, 2, 3, 4]; str(xs.take first 3)", "[1 2 3]");
	is!("xs = [1, 2, 3, 4]; count(xs.keep only even)", 2);
}

#[test]
fn sort_by_an_expression_of_it() {
	is!("xs = [3, 1, 2]; xs.sort by it", ints(vec![1, 2, 3]));
	is!("xs = [3, 1, 2]; xs.sort by -it", ints(vec![3, 2, 1]));
}
