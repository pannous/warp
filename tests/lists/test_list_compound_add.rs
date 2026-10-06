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
	crate::common::fails_with("xs = [1]; xs += 3; xs", "lists only concatenate with lists");
	// `[]` is ø, of no kind yet: `xs += 3` fails as `xs = xs + 3` does
	crate::common::fails_with("xs = []; xs += 3; xs", "not a number");
}

#[test]
fn a_function_is_no_number() {
	crate::common::fails_with("1 + (x => x)", "type error: int + function");
	crate::common::fails_with("fs = []; fs += (x => x * 2); fs", "no implicit conversion");
}
