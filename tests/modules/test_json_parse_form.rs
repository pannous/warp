//! card g_ogQg: `json.parse(t)` is how warp writes it, bringing `use json` itself; `parse_json(t)` stays an alias with
//! a got-it note naming json.parse, as do JS's `JSON.parse(t)` and Python's `json.loads(t)`
use crate::is;
use warp::normalize::{capture_hints, clear_shown_hints};
use warp::warp_parser::parse;

fn hints_of(code: &str) -> Vec<(String, String)> {
	clear_shown_hints();
	let (_, hints) = capture_hints(|| warp::wasm_emitter::eval(code));
	hints.into_iter().map(|hint| (hint.original, hint.canonical)).collect()
}

#[test]
fn json_parse_reads_json() {
	is!("json.parse(\"[1, 2]\")", parse("[1 2]"));
	is!("use json; json.parse(\"{\\\"age\\\": 30}\").age", 30);
	assert_eq!(hints_of("json.parse(\"[1]\")"), vec![]);
}

#[test]
fn parse_json_is_an_alias_of_json_parse() {
	is!("use json; parse_json(\"[1, 2]\")", parse("[1 2]"));
	assert!(hints_of("use json; parse_json(\"[1]\")").contains(&("parse_json".into(), "json.parse".into())));
	assert!(hints_of("JSON.parse(\"[1]\")").contains(&("JSON.parse".into(), "json.parse".into())));
}
