//! What samples/json_parser.wasp needed: fields of what a function returns, nested lists as text
use warp::*;

#[test] // `result = parse_json(text); result.name` was "undefined function: name"
fn test_field_of_a_call_result() {
	is!("def make() := {name: \"Alice\", age: 30}; result = make(); result.name", "Alice");
	is!("def make() { m = {}; m[\"k\"] = 7; return m }; r = make(); r.k", 7);
}

#[test] // a list inside a joined or converted list is its text "[1 2]"
fn test_nested_list_as_text() {
	is!("x = [[1, 2], [3]]; x.join(\", \")", "[1 2], [3]");
	is!("x = [1, [2, [3, 4]]]; str(x)", "[1 [2 [3 4]]]");
}
