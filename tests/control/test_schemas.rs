//! A map pattern of types is a schema: `pattern Config = {port: int, host: text, debug?: bool}`, then
//! `json(text) is Config`; a field `debug?: bool` may be missing, and when there it has the type (card schemas,
//! tree-patterns step 6)
use crate::is;

const CONFIG: &str = "pattern Config = {port: int, host: text, debug?: bool}\n";

fn config_test(value: &str) -> String {
	format!("{CONFIG}{value} is Config")
}

#[test]
fn a_map_of_types_tests_the_field_types() {
	is!("{port: 80, host: \"x\"} is {port: int, host: text}", true);
	is!("{port: \"80\", host: \"x\"} is {port: int, host: text}", false);
	is!("{port: 80} is {port: int, host: text}", false);
}

#[test]
fn an_optional_field_may_be_missing() {
	is!(&config_test("{port: 80, host: \"x\"}"), true);
	is!(&config_test("{port: 80, host: \"x\", debug: yes}"), true);
	is!(&config_test("{port: 80, host: \"x\", debug: 3}"), false);
}

#[test]
fn parsed_json_is_tested_against_a_schema() {
	is!(&config_test("json('{\"port\": 80, \"host\": \"x\"}')"), true);
	is!(&config_test("json('{\"port\": \"80\", \"host\": \"x\"}')"), false);
}

#[test]
fn an_arm_matches_a_schema() {
	is!(&format!("{CONFIG}match {{port: 80, host: \"x\"}} {{ Config => \"config\"; _ => \"other\" }}"), "config");
}
