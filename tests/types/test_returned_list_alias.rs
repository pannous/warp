//! A list a function returns as it got it is the caller's list: a change through the result shows in the variable
//! passed (P208, card identity-copies-list)
use crate::is;

#[test]
fn test_change_through_a_returned_list() {
	is!("f(a) := a; xs = [1, 2]; ys = f(xs); ys#1 = 7; xs", warp::ints(vec![7, 2]));
	is!("f(a) := a; xs = [1, 2]; ys = f(xs); ys.add(3); count xs", 3);
	is!("f(a) := { return a }; xs = [1, 2]; ys = f(xs); ys#2 = 9; xs#2", 9);
	is!("f(a) := a; xs = [1, 2]; ys = f(xs); ys === xs", true);
}

#[test]
fn test_fresh_result_stays_apart() {
	is!("f(a) := [1, 2]; xs = [1, 2]; ys = f(xs); ys#1 = 7; xs", warp::ints(vec![1, 2]));
}
