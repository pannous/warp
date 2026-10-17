//! card inferred-kinds (notes/self_hosting.md step 14): the analyzer's warp twin, lib/compiler/analyzer.warp, gets the
//! program after every lowering pass and writes the kinds of its functions, variables and value as text, compared with
//! src/analyzer/inferred_kinds.rs (tests/common/twins.rs analyzer_differential)
use crate::common::twins::{analyzed, analyzer_differential, sources, view_json};
use std::path::PathBuf;

/// The sources whose kinds the warp analyzer infers as the Rust one does, one path per line: the list only grows
const MATCHING_SOURCES: &str = "tests/parser/analyzer_twin_sources.txt";
use warp::analyzer::inferred_kinds;
use warp::pipeline::lower;

fn kinds_of(code: &str) -> Vec<String> {
	inferred_kinds(&lower(code).expect("a program needing a module"))
}

#[test]
fn kinds_of_functions_variables_and_value() {
	assert_eq!(kinds_of("f(a) := sqrt(a) * 2\nx = f(2)\nname = 'pi'\nx"), [
		"f(int) -> float",
		"name: text",
		"x: float",
		"= float"
	]);
}

#[test]
fn matching_sources_keep_matching() {
	let listed = std::fs::read_to_string(MATCHING_SOURCES).expect("the list of matching sources");
	let paths: Vec<PathBuf> = listed.lines().filter(|line| !line.is_empty()).map(PathBuf::from).collect();
	analyzer_differential(&paths);
}

/// The cases probes/self_hosting/analyzer/twin_loop.py checks without a cargo run: path, Rust kinds, program view
#[test]
#[ignore = "writes data/self_analyzer/cases.jsonl (~9 min); run with --ignored"]
fn survey_rust_kinds() {
	let mut out = String::new();
	for path in sources() {
		if let Some((program, kinds)) = analyzed(&std::fs::read_to_string(&path).unwrap()) {
			out += &format!("{{\"path\": {:?}, \"kinds\": {:?}, \"program\": {}}}\n", path.display().to_string(), kinds, view_json(&program));
		}
	}
	std::fs::create_dir_all("data/self_analyzer").unwrap();
	std::fs::write("data/self_analyzer/cases.jsonl", out).unwrap();
}
