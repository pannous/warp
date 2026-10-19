//! The self-hosted analysis stage (card End-end): analyzer.warp `kinds` of a lowered program, as the JSON data the
//! Rust analyzer's self-hosted hand-off maps
use warp::compiler::node_views::plain_view;
use warp::compiler::self_hosted::kinds_of;

#[test]
fn the_analysis_stage_gives_the_kinds_as_data() {
	let program = warp::pipeline::lower("square(x: int) := x * x; y = square(3); \"square: ${y}\"").expect("lowers");
	let kinds = kinds_of(&plain_view(&program)).expect("the analysis stage answers");
	assert_eq!(kinds["functions"]["square"]["params"], serde_json::json!(["int"]), "{kinds}");
	assert_eq!(kinds["functions"]["square"]["result"], "int", "{kinds}");
	assert_eq!(kinds["variables"]["y"], "int", "{kinds}");
	assert_eq!(kinds["value"], "text", "{kinds}");
}
