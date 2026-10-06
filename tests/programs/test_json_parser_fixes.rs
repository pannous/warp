//! What samples/json_parser.wasp needed: fields of what a function returns, nested lists as text
use crate::is;

#[test] // `result = parse_json(text); result.name` was "undefined function: name"
fn test_field_of_a_call_result() {
	is!("def make() := {name: \"Alice\", age: 30}; result = make(); result.name", "Alice");
	is!("def make() { m = {}; m[\"k\"] = 7; return m }; r = make(); r.k", 7);
}

#[test] // the text of a value known only at run time: a list is "[1 2]" (decision #35's form); join still refuses lists in lists
fn test_text_of_a_run_time_value() {
	is!("m = {a: [1, 2]}; str(m.a)", "[1 2]");
	is!("m = {a: 5}; str(m.a)", "5");
	crate::common::fails_with("join([[1], [2]], \",\")", "not a joinable item");
}

#[test] // a function returning texts, lists or maps returns a Node of unknown kind, not a text
fn test_mixed_returns_are_nodes() {
	is!("def f(n) { if n == 1 { return \"a\" }; return [1, 2] }; x = f(2); count(x)", 2);
}
