//! The standard library module json (notes/stdlib.md section 7): `use json` loads std/json.wasp, whose words call the
//! adapter std_pure: serde_json natively, JSON.parse / JSON.stringify in the browser
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn use_json_parses_and_writes_json() {
	is!("use json; parse_json(\"[1, 2, 3]\")", parse("[1 2 3]"));
	is!("use json; data = parse_json(\"{\\\"name\\\": \\\"Ann\\\", \\\"age\\\": 30}\"); data.age", 30);
	is!("use json; to_json([1, 2, 3])", "[1,2,3]");
	is!("use json; to_json(parse_json(\"{\\\"a\\\": [true, null]}\"))", "{\"a\":[1,null]}");
}

#[test]
fn a_json_word_without_its_use_names_the_module() {
	crate::common::fails_with("parse_json(\"[1]\")", "parse_json is in the standard module json: write `use json`");
}

#[test]
fn malformed_json_is_an_error_naming_it() {
	crate::common::fails_with("use json; parse_json(\"[1, 2\")", "json");
}
