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

/// card parse-json: quoted code keeps its operations, each as `{operator: [operands]}` (the JsonLogic shape)
#[test]
fn json_of_code_keeps_its_operations() {
	is!("parse(\"1+1\").json", "{\"+\":[1,1]}");
	is!("parse(\"a * (b + 1)\").json", "{\"*\":[\"a\",{\"+\":[\"b\",1]}]}");
	is!("parse(\"not x\").json", "{\"not\":[\"x\"]}");
	is!("parse(\"{a: 1+2}\").json", "{\"a\":{\"+\":[1,2]}}");
}

/// the browser names a key's operator by its code (host.js OPERATOR_SPELLINGS), as the native OP_CODES do
#[test]
fn the_browser_spells_each_operator_code_alike() {
	let host = include_str!("../../web/playground/host.js");
	let line = host.lines().find(|line| line.starts_with("const OPERATOR_SPELLINGS = ")).expect("the table");
	let spellings: Vec<&str> = line.split('"').skip(1).step_by(2).collect();
	let native: Vec<&str> = warp::operators::OP_CODES.iter().map(|op| op.as_str()).collect();
	assert_eq!(spellings, native);
}

/// user 2026-10-15 (card parse-json): what JSON cannot express is never lost silently: any key is written as its text,
/// a number JSON has no form for is an error
#[test]
fn json_loses_nothing_silently() {
	is!("{1: \"a\"}.json", "{\"1\":\"a\"}");
	is!("parse(\"f(x) := x*2\").json", "{\"(f x)\":{\"*\":[\"x\",2]}}");
	assert!(warp::wasm_emitter::eval("(1/0.0).json").serialize().contains("JSON has no"));
	assert!(warp::wasm_emitter::eval("[1 (0/0.0)].json").serialize().contains("JSON has no NaN"));
}

/// decision json-exactness: a big integer keeps all its digits; a fraction is JSON's float with a warning naming the
/// loss, under `use strict` an error
#[test]
fn json_writes_numbers_exactly_or_says_so() {
	is!("(2^70).json", "1180591620717411303424");
	is!("[-(2^70) 1].json", "[-1180591620717411303424,1]");
	is!("(1/3).json", "0.3333333333333333");
	assert!(warp::wasm_emitter::eval("use strict\n(1/3).json").serialize().contains("JSON has no exact 1/3"));
}

#[cfg(feature = "native")]
#[test]
fn a_fraction_in_json_warns_of_its_loss() {
	warp::errors::diagnostic::take_runtime_warnings();
	is!("{third: 1/3}.json", "{\"third\":0.3333333333333333}");
	let warnings = warp::errors::diagnostic::take_runtime_warnings();
	assert!(warnings.iter().any(|warning| warning.contains("JSON has no exact 1/3")), "{warnings:?}");
}
