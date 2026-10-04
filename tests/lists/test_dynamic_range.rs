// A range with computed bounds is a list wherever a value is needed, as a range of literals is
use warp::is;

#[test]
fn a_range_with_variable_bounds_is_a_list() {
	is!("a=1; b=5; sum(a..b)", 10);
	is!("a=1; b=5; xs = a..b; #xs", 4);
	is!("range_sum(a, b) := sum(a..b); range_sum(1, 5)", 10);
	is!("n=3; xs = 0 to n; xs#4", 3);
}

#[test]
fn a_list_grows_from_empty_by_adding_lists() {
	is!("out = []; xs=[2]; out = out + xs; out", warp::ints(vec![2]));
	is!("flatten(xs) := { out = []; for x in xs { out = out + x }; out }; flatten([[1 2] [3] [4 5]])", warp::ints(vec![1, 2, 3, 4, 5]));
	is!("f(a, b) := a + b; f([1], [2])", warp::ints(vec![1, 2]));
}

#[test]
fn a_recursive_list_function_in_a_ternary() {
	is!("f(xs) := #xs <= 1 ? xs : xs.filter(x => x > 1); f([1 2 3])", warp::ints(vec![2, 3]));
	is!("f(xs) := #xs <= 1 ? xs : f(xs[1:]) + [xs#1]; f([1 2 3])", warp::ints(vec![3, 2, 1]));
	is!("qs(xs) := #xs <= 1 ? xs : qs(xs.filter(x => x < xs#1)) + xs.filter(x => x == xs#1) + qs(xs.filter(x => x > xs#1)); qs([3 1 4 1 5 9 2 6])", warp::ints(vec![1, 1, 2, 3, 4, 5, 6, 9]));
}
