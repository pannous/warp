// card count-of-field: `print #s.names` counts the field, print(#s.names), never the field names of print's result
use crate::is;

#[test]
fn a_count_of_a_field_after_a_function() {
	is!("class S { names: list = [] }; s = S(); f(n) := n + 1; f #s.names", 1);
	is!("class S { names: list = [1, 2] }; s = S(); f(n) := n * 10; f #s.names", 20);
}

#[cfg(feature = "native")]
#[test]
fn print_a_count_of_a_field() {
	assert_eq!(crate::common::printed("class S { names: list = [] }\ns = S()\nprint #s.names"), "0\n");
}
