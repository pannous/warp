//! card alias-list-type: a type alias names a list type, a plural type word or a type phrase
use crate::is;

#[test]
fn an_alias_of_a_plural_type_word() {
	is!("type Code = ints\nf(c: Code) := #c\nf([1, 2])", 2);
}

#[test]
fn an_alias_of_a_type_phrase() {
	is!("type Code = list of int\nf(c: Code) := #c\nf([1, 2])", 2);
	is!("type Code = list of int\ntype Program = Code\nf(c: Program) := #c\nf([1, 2])", 2);
}
