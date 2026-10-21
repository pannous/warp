// `xs += y` is `xs = xs + y` (cards xs-xs, fs-fs): a list concatenates with a list; anything else is the same clear
// type error as `xs + 3`, never an invalid module
use crate::is;

#[test]
fn a_list_plus_equals_a_list_concatenates() {
	is!("xs = [1, 2]; xs += [3]; xs", warp::ints(vec![1, 2, 3]));
	is!("fs = []; fs += [x => x * 2]; fs#1(4)", 8);
}

#[test]
fn a_list_plus_equals_an_item_is_the_type_error_of_plus() {
	is!("xs = [1]; xs += 3; xs", warp::ints(vec![1, 3]));
	is!("xs = []; xs += 3; xs", warp::ints(vec![3]));
}

#[test]
fn a_function_is_no_number() {
	crate::common::fails_with("1 + (x => x)", "type error: int + function");
	is!("fs = []; fs += (x => x * 2); fs#1(4)", 8);
}
