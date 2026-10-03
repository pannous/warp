// The compiler in the browser (web/playground): evaluate reports what the CLI prints plus the page's Asks,
// and a run in the page comes back as a JSON tree read through the module's reflection exports
use serde_json::json;
use std::collections::HashMap;
use warp::web::{evaluate, node_from_tree, run_outcome};
use warp::Node;

const UPTO_LOOP: &str = "x=0; for i in 1 upto 4 {x+=i}; x";

fn answers(pairs: &[(&str, &str)]) -> HashMap<String, String> {
	pairs.iter().map(|(topic, form)| (topic.to_string(), form.to_string())).collect()
}

#[test]
fn evaluate_reports_the_value_the_cli_prints() {
	let report = evaluate("3+3", HashMap::new());
	assert_eq!(report["value"], "6");
	assert_eq!(report["error"], false);
	assert_eq!(evaluate("[1 2.5 'x' (a:3)]", HashMap::new())["value"], "[1 2.5 'x' a:3]");
	assert_eq!(evaluate("xs=[1 2]; xs#5", HashMap::new())["error"], true);
}

#[test]
fn an_unanswered_ask_is_reported_with_its_readings_and_falls_back() {
	let report = evaluate(UPTO_LOOP, HashMap::new());
	assert_eq!(report["value"], "6");
	let ask = &report["asks"][0];
	assert_eq!(ask["topic"], "upto");
	assert!(ask["readings"].as_array().unwrap().len() >= 2);
	assert!(report["warnings"][0]["message"].as_str().unwrap().contains("taking"));
}

#[test]
fn the_pages_answer_is_taken_and_hinted() {
	let report = evaluate(UPTO_LOOP, answers(&[("upto", "...")]));
	assert_eq!(report["value"], "10");
	assert_eq!(report["asks"], json!([]));
	assert!(report["hints"].as_array().unwrap().iter().any(|hint| hint["canonical"] == "..."));
}

#[test]
fn a_tree_from_the_page_reads_like_from_gc_object() {
	let int = |n: i64| json!({"kind": "1", "data": {"int": n.to_string()}});
	let square_list = (1i64 << 8 | 8).to_string();
	let list = json!({"kind": square_list, "data": {"node": int(1)}, "chain": [
		{"kind": square_list, "data": {"node": {"kind": "3", "data": {"text": "x"}}}},
	]});
	assert_eq!(node_from_tree(&list).serialize(), r#"[1 "x"]"#);
	let big = json!({"kind": "1", "data": {"big": {"negative": true, "limbs": [0, 1]}}});
	assert_eq!(node_from_tree(&big).serialize(), "-4294967296");
	let ratio = json!({"kind": "1", "data": {"ratio": [{"int": "1"}, {"int": "3"}]}});
	assert_eq!(node_from_tree(&ratio).serialize(), "1/3");
	assert_eq!(node_from_tree(&json!({"kind": "2", "data": {"float": "-Infinity"}})), Node::Number(warp::Number::Float(f64::NEG_INFINITY)));
}

#[test]
fn a_trap_in_the_page_names_the_runtime_error() {
	let trap = json!({"trap": "unreachable", "trace": "RuntimeError: unreachable\n    at index_out_of_range (wasm://wasm/1:1)"});
	assert!(matches!(run_outcome(&trap), Node::Error(_)));
	assert!(run_outcome(&trap).serialize().contains("index out of range"));
	assert!(run_outcome(&json!({"failure": "link error"})).serialize().contains("could not run the program"));
}

#[test]
fn a_returned_value_has_its_own_kind_on_every_platform() {
	// the browser has no C headers: `return` used to be typed by a bogus libm "function" parsed from math.h
	assert!(!warp::ffi::is_ffi_function("return"));
	warp::is!("fun h(){ return 5 }\nh() + 3", 8);
	warp::is!("fun h(){ return 2.5 }\nh() * 2", 5.0);
}
