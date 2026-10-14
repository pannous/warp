//! A unit word in a pattern is the unit: `{port: p, host: h}` compares host with the hour and binds nothing, with a
//! weak warning that names the unit and the way to bind (card pattern-unit-name)
use crate::common::{warnings_of, warns_with};

const UNIT_HINT: &str = "h is the unit hour";

#[test]
fn a_unit_word_as_a_field_pattern_warns() {
	warns_with("{port: 80, host: \"x\"} is {port: p, host: h}", false, UNIT_HINT);
	warns_with("[{host: \"x\"}] is [{host: h}]", false, UNIT_HINT);
}

#[test]
fn a_list_item_named_like_a_unit_binds() {
	let (value, warnings) = warnings_of("xs = [1, 2]\nif xs is [_, h] then h else 0");
	assert_eq!(value, warp::Node::int(2));
	assert!(!warnings.iter().any(|warning| warning.contains(UNIT_HINT)), "{warnings:?}");
}

#[test]
fn a_dollar_name_binds_without_a_warning() {
	let (value, warnings) = warnings_of("c = {host: \"x\"}\nif c is {host: $h} then $h else \"none\"");
	assert_eq!(value, warp::Node::Text("x".into()));
	assert!(!warnings.iter().any(|warning| warning.contains(UNIT_HINT)), "{warnings:?}");
}

#[test]
fn a_program_variable_named_like_a_unit_is_no_unit() {
	let (_, warnings) = warnings_of("h = 2\n{a: 2} is {a: h}");
	assert!(!warnings.iter().any(|warning| warning.contains(UNIT_HINT)), "{warnings:?}");
}

#[cfg(feature = "native")] // printed runs the warp binary
#[test] // a bound `$h` is an argument like any name: `then print $h` prints the value (card if-host)
fn a_dollar_name_is_a_call_argument() {
	assert_eq!(crate::common::printed("c = {host: \"x\"}\nif c is {host: $h} then print $h"), "x\n");
}
