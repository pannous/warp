//! A variable first holding a one-letter text is read back as that letter, also after reassignment

use crate::is;

#[test]
fn a_one_letter_text_variable_reads_back_as_a_letter() {
	is!("x=\"a\"; x", "a");
	is!("y=\"a\"; x=y; x", "a");
}

#[test]
fn a_one_letter_text_can_be_reassigned() {
	is!("x=\"a\"; x=\"b\"; x", "b");
	is!("x=\"a\"; if 1 {x=\"b\"}; x", "b");
	is!("x='a'; if 1 {x='b'}; x", "b");
}
