// card error-local: a local holding a plain value applied like a function, `n = 2; n(3)`, is one error naming the
// variable, for every name (the counting words included, test_count_shadowed); before, it said only "undefined function: n"
use crate::common::fails_with;
use crate::is;

#[test]
fn a_local_holding_a_plain_value_applied_to_a_value_is_an_error() {
	fails_with(
		"n = 2; n(3)",
		"undefined function: n (`n` is a variable here, so n(3) calls nothing: rename the variable)",
	);
	fails_with("s = \"ab\"; s(1)", "`s` is a variable here, so s(1) calls nothing");
	fails_with("x = 1.5; y = x(2); y", "`x` is a variable here, so x(2) calls nothing");
	fails_with("xs = [1, 2]; xs(0)", "`xs` is a variable here, so xs(0) calls nothing");
}

#[test]
fn locals_holding_functions_and_data_lists_stay() {
	is!("x = 1; [x x]", warp::ints(vec![1, 1]));
	is!("n = 2; twice(x) := x * 2; twice(n)", 4);
	is!("add(a) := b => a + b; add(2)(5)", 7);
	is!("inc := {it + 1}; inc(2)", 3);
}
