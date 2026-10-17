//! P45 for functions (card lambda-reassign): a name bound to a function, a lambda included, given a value of another
//! kind is a compile error naming both kinds, as `m = 5; m = "a"` is; another function for the name stays allowed
use crate::common::fails_with;
use crate::is;

#[test]
fn a_function_name_given_another_kind_is_a_compile_error() {
	fails_with("f = x => x*2; f = 3; 0", "f was a function, is given an Int: use another name");
	fails_with("f = x => x*2; f = [1]; 0", "f was a function, is given a List");
	is!("f = x => x*2; f = y => y+1; f(2)", 3);
	is!("f = x => x*2; g = 3; f(g)", 6);
}
