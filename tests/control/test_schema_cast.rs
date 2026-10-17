//! `x as Config` with `pattern Config = {port: int, host: text, debug?: bool}`: x when it fits, else x converted field
//! by field where `as` converts, else the error naming the field; `x is Config` stays the strict test (card
//! loaded-config)
use crate::common::fails_with;
use crate::is;

const CONFIG: &str = "pattern Config = {port: int, host: text, debug?: bool}\n";

fn cast(value: &str, read: &str) -> String {
	format!("{CONFIG}c = {value} as Config\n{read}")
}

#[test]
fn a_fitting_value_is_itself() {
	is!(&cast("json('{\"port\": 8080, \"host\": \"localhost\"}')", "c.port"), 8080);
}

#[test]
fn fields_convert_where_as_converts() {
	is!(&cast("{port: \"80\", host: \"x\"}", "c.port + 1"), 81);
	is!(&cast("{port: \"80\", host: 8080, debug: yes}", "c.host"), "8080");
}

#[test]
fn a_missing_optional_field_stays_missing() {
	is!(&cast("{port: \"80\", host: \"x\"}", "c has debug"), false);
}

#[test]
fn the_original_is_not_changed() {
	is!(&format!("{CONFIG}m = {{port: \"80\", host: \"x\"}}\nc = m as Config\nm.port"), "80");
}

#[test]
fn a_field_that_does_not_convert_is_named() {
	fails_with(&cast("{port: \"abc\", host: \"x\"}", "c"), "port of Config takes an int, got \"abc\"");
	fails_with(&cast("{host: \"x\"}", "c"), "port of Config is missing");
}
