//! `x.f` and `x.f(y)` call the user function `f` with the receiver as its first argument

use warp::is;

#[test]
fn a_user_function_is_called_in_method_form() {
	is!("square(x) := x*x; x=4; x.square", 16);
	is!("square := it*it; x=4; x.square", 16);
}

#[test]
fn a_method_form_call_passes_the_further_arguments() {
	is!("add(a, b) := a+b; x=4; x.add(3)", 7);
	is!("square(x) := x*x; x=4; x.square()", 16);
}

#[test]
fn a_user_function_taking_a_function_in_method_form() {
	warp::is!("def map(list, fn) { result = []; for item in list { result.push(fn(item)) }; return result }; square := it * it; [1, 2, 3].map(square)", warp::ints(vec![1, 4, 9]));
}
