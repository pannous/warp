//! card self-serializer: Node::serialize written in warp (lib/compiler/serializer.warp) writes every sample and probe
//! as its Rust twin does, from the node view the parser twin shares (tests/common/twins.rs node_view)
use crate::common::twins::{node_view, view_differential, view_json};
use warp::warp_parser::parse;

#[test]
fn a_view_is_json() {
	let json = view_json(&node_view(&parse("say \"a \\\"b\\\"\"\nx: ø")));
	assert!(json.contains(r#"["text", "a \"b\""]"#), "{json}");
	assert!(json.contains(r#"["key", ":", ["symbol", "x"], null]"#), "{json}");
}

#[test]
fn serialize_matches_its_rust_twin() {
	view_differential("serializer", "serialize");
}
