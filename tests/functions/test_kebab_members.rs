//! P168 (user): `p.phone-number` of an object without that field, `number` a variable, is `p.phone - number`
use crate::is;

#[test]
fn a_missing_kebab_field_subtracts_the_variable() {
	is!("p = {phone: 7}; number = 2; p.phone-number", 5);
}

#[test]
fn an_existing_kebab_field_is_read() {
	is!("p = {phone-number: 7}; number = 2; p.phone-number", 7);
	is!("p = {phone-number: 7}; p.phone-number", 7);
}
