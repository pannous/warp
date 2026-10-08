//! A variable given lists of two element types holds items of both (card list-retype: `ys = ["a"]; ys = [3]; ys#1 + 1`
//! trapped "cast failure", the variable kept the element type of its first list)
use crate::is;

#[test]
fn a_list_variable_given_another_element_type_holds_both() {
	is!("ys = [\"a\"]; ys = [3]; ys#1 + 1", 4);
	is!("ys = [3]; ys = [\"a\"]; ys#1 is text", true);
	is!("ys = [1]; ys = [2.5]; ys#1 + 1", 3.5);
	is!("ys = [1]; ys = [2]; ys#1 + 1", 3);
}
