//! `"1/3" as number` is the exact 1/3, as the code 1/3 is; a text that spells no number is the error invalid number,
//! never a silent 0 (card fraction-number)
use crate::common::fails_with;
use crate::is;

#[test]
fn a_fraction_text_as_number_is_exact() {
	is!("'1/3' as number == 1/3", true);
	is!("('1/3' as number) * 3", 1);
	is!("'-6/4' as number == -3/2", true);
	is!("'4/2' as number", 2);
}

#[test]
fn a_text_that_spells_no_number_fails_as_number() {
	fails_with("'abc' as number", "invalid number");
	fails_with("'1/x' as number", "invalid number");
}
