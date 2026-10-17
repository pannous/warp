//! field-or-minus (user 2026-10-12, replaced P168's subtraction): `p.phone-number` of an object without that field,
//! `number` a variable, is an error offering `p.phone - number`
use crate::common::fails_with;
use crate::is;

#[test]
fn a_missing_kebab_field_is_an_error_offering_the_subtraction() {
	fails_with("p = {phone: 7}; number = 2; p.phone-number", "p has no field phone-number");
	is!("p = {phone: 7}; number = 2; p.phone - number", 5);
}

#[test]
fn an_existing_kebab_field_is_read() {
	is!("p = {phone-number: 7}; number = 2; p.phone-number", 7);
	is!("p = {phone-number: 7}; p.phone-number", 7);
}
