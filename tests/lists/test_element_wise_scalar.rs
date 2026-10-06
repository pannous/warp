//! P166 (user): an element-wise operator on a plain number is the plain operator; a list still maps
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn element_wise_on_a_number_is_the_plain_operator() {
	is!("6 ./ 2", 3);
	is!("sq = @(x) x.^2; sq(3)", 9);
	is!("f(x) := x .+ 1; f(4)", 5);
	is!("add = @(a, b) a .* b; add(3, 4)", 12);
}

#[test]
fn element_wise_on_a_list_still_maps() {
	is!("sq = @(x) x.^2; sq([1,2,3])", parse("[1 4 9]"));
	is!("f(x) := x .+ 1; f([4,5])", parse("[5 6]"));
	is!("xs=[1,2]; xs .* 3", parse("[3 6]"));
	is!("g(xs) := count(xs) + (xs .* 2)#1; g([5,6])", 12);
}
