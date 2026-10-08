//! A bool variable takes only bools (card bool-assign): int ≰ bool, while bool ≤ int lets an int variable take one
use crate::common::fails_with;
use crate::is;

#[test]
fn a_bool_variable_refuses_other_kinds() {
	fails_with("b = true; b = 2; b", "b was a Bool, is given an Int");
	fails_with("b = true; b = \"a\"; b", "b was a Bool, is given a Text");
	fails_with("b = 1 > 0; b = 3; b", "b was a Bool, is given an Int");
	fails_with("b: bool = true; b = 2; b", "b is declared bool, cannot assign int 2");
	fails_with("b: bool = 1", "b is declared bool, cannot assign int 1");
}

#[test]
fn a_bool_variable_takes_bools_and_an_int_variable_takes_a_bool() {
	is!("b = true; b = false; b", false);
	is!("b = true; b = 2 > 3; b", false);
	is!("b: bool = true; b = no; b", false);
	is!("n = 2; n = true; n + 1", 2);
	is!("n: int = true; n + 1", 2);
}
