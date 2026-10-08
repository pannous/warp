//! Unpacking into names that already hold numbers assigns them (card destructure-existing; it failed at run time with
//! "a holds Int, the destructured value is Data"); an item of another kind is the run-time error naming the kind
use crate::common::fails_with;
use crate::is;

#[test]
fn unpacking_assigns_names_that_hold_numbers() {
	is!("a = 0; b = 0; a, b = [5, 6]; a + b", 11);
	is!("a = 0; b = 0; (a, b) = (5, 6); a + b", 11);
	is!("x = 0.5; y = 1; x, y = [2.5, 3]; x + y", 5.5);
	is!("x = 0.5; x, y = [2, 3]; x + y", 5.0);
	fails_with("a = 0; a, b = [\"x\", 1]; a", "not an int");
}
