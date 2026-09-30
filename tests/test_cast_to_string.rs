//! `value as string` is the serialized text of a literal; a value only known at runtime is a loud error, never a panic

mod common;
use common::fails_with;
use warp::is;

#[test]
fn a_list_literal_as_string_is_its_text() {
	is!("[1 2] as string", "[1 2]");
}

#[test]
fn an_object_literal_as_string_is_its_text() {
	is!("{a:1} as string", "{a:1}");
}

#[test]
fn a_variable_holding_a_list_as_string_is_an_error_value() {
	fails_with("x=[1 2]; x as string", "as string");
}
