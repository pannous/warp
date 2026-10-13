//! card self-parser: the parser written in warp (lib/compiler/parser.warp) next to the Rust parser, both trees seen as
//! node_view (tests/common/twins.rs). The sources it parses as Rust does are listed in PARSED_SOURCES, which only grows;
//! `parser_progress` (ignored) names the next ones.
use crate::common::twins::{around_difference, differences, differential_on, node_view, sources};
use std::path::PathBuf;
use warp::Node;
use warp::WarpParser;

/// The sources parser.warp reads as the Rust parser does, relative to the crate
const PARSED_SOURCES: &[&str] = &["probes/self_hosting/parser/data_core.warp"];
/// Differences shown by parser_progress
const SHOWN: usize = 20;

fn rust_tree(text: &str) -> Node {
	node_view(&WarpParser::parse(text))
}

#[test]
fn the_listed_sources_parse_as_in_rust() {
	let paths: Vec<PathBuf> = PARSED_SOURCES.iter().map(PathBuf::from).collect();
	differential_on(&paths, "parser", "tree", rust_tree);
}

/// How many of all sources parse as in Rust, and the first differences: `tests/queue.sh -- parser_progress --ignored
/// --nocapture`
#[test]
#[ignore]
fn parser_progress() {
	let all = sources();
	let differing = differences(&all, "parser", "tree", rust_tree);
	for (path, warp_text, rust_text) in differing.iter().take(SHOWN) {
		println!(
			"{path}\n  warp: …{}…\n  rust: …{}…",
			around_difference(warp_text, rust_text),
			around_difference(rust_text, warp_text)
		);
	}
	println!(
		"{} of {} sources parse as in Rust",
		all.len() - differing.len(),
		all.len()
	);
}
