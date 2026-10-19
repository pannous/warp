// card fixnow-builtin-function-add: a list given to a function value may be changed there, as any function may change
// the list it gets, so it stays the one Node list (main's typed-list pass no longer sees the function's own body)
use crate::is;

#[test]
fn a_list_passed_to_a_function_value_is_changed_in_place() {
	is!("f = function add; xs = [1]; f(xs, 2); xs", warp::ints(vec![1, 2]));
	is!("g = (ys, v) => ys.push(v); xs = [1]; g(xs, 2); xs", warp::ints(vec![1, 2]));
	is!("push2(ys) := ys.push(2); f = function push2; xs = [1, 3]; f(xs); xs#3", 2);
}
