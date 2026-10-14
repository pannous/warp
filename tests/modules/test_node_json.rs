//! Card node-json (user: "node.json available on all nodes"): `x.json` is the JSON text of any value, without a
//! `use json`; a field named json still wins
use crate::is;

#[test]
fn json_of_a_value_is_its_json_text() {
	is!("{a: 1}.json", "{\"a\":1}");
	is!("x = {a: 1, b: [1, 2]}; x.json", "{\"a\":1,\"b\":[1,2]}");
	is!("[1, 2, 3].json", "[1,2,3]");
	is!("\"hi\".json", "\"hi\"");
	is!("(1 + 2).json", "3");
}

#[test]
fn a_field_named_json_wins() {
	is!("x = {json: 3}; x.json", 3);
}
