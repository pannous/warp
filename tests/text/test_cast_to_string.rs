//! `value as string` is the serialized text of a literal; an int list known only at runtime joins to "[1 2]" (user decision #35)

use crate::is;

#[test]
fn a_list_literal_as_string_is_its_text() {
	is!("[1 2] as string", "[1 2]");
}

#[test]
fn an_object_literal_as_string_is_its_text() {
	is!("{a:1} as string", "{a:1}");
}

#[test]
fn a_variable_holding_an_int_list_as_string_is_its_text() {
	is!("x=[1 2]; x as string", "[1 2]"); // was an error value until user decision #35
}
