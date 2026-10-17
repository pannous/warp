//! card self-parser: the parser written in warp (lib/compiler/parser.warp) next to the Rust parser, both trees seen as
//! node_view (tests/common/twins.rs). The sources it parses as Rust does are listed in PARSED_SOURCES, which only grows;
//! `parser_progress` (ignored) names the next ones and writes all that match to MATCHING.
use crate::common::twins::{around_difference, differences, differential_on, node_view, sources};
use std::path::PathBuf;
use warp::Node;
use warp::WarpParser;

/// The sources parser.warp reads as the Rust parser does, one path per line relative to the crate
pub(super) const PARSED_SOURCES: &str = "tests/parser/self_parser_sources.txt";
/// Every source parser.warp reads as the Rust parser does, written by parser_progress
const MATCHING: &str = "data/self_parser/matching.txt";
/// One form per line that parser.warp is learning, shown by rust_forms
const FORMS: &str = "probes/self_hosting/parser/forms.warp";
/// The forms of several lines, ⏎ and ⇥ writing their line breaks and tabs (not a source: Rust reads ⏎ as an operator)
const BLOCK_FORMS: &str = "tests/parser/self_parser_block_forms.txt";
/// Differences shown by parser_progress
const SHOWN: usize = 20;

fn rust_tree(text: &str) -> Node {
	node_view(&WarpParser::parse(text))
}

fn lines_of_form(line: &str) -> String {
	line.replace('⏎', "\n").replace('⇥', "\t")
}

#[test]
fn the_listed_sources_parse_as_in_rust() {
	let listed = std::fs::read_to_string(PARSED_SOURCES).unwrap();
	let paths: Vec<PathBuf> = listed.lines().map(PathBuf::from).collect();
	differential_on(&paths, "parser", "tree", rust_tree);
}

/// The Rust tree of each line of FORMS, as parser.warp is to write it: `tests/queue.sh -- rust_forms --ignored
/// --nocapture`
#[test]
#[ignore]
fn rust_forms() {
	let forms = std::fs::read_to_string(FORMS).unwrap() + &std::fs::read_to_string(BLOCK_FORMS).unwrap();
	for line in forms.lines() {
		println!("{line}\n  {}", rust_tree(&lines_of_form(line)).serialize().replace('\n', "⏎"));
	}
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
	let differing_paths: Vec<&str> = differing.iter().map(|(path, _, _)| path.as_str()).collect();
	let matching: Vec<String> = all
		.iter()
		.map(|path| path.display().to_string())
		.filter(|path| !differing_paths.contains(&path.as_str()))
		.collect();
	std::fs::create_dir_all(std::path::Path::new(MATCHING).parent().unwrap()).unwrap();
	std::fs::write(MATCHING, matching.join("\n") + "\n").unwrap();
	println!("{} of {} sources parse as in Rust, listed in {MATCHING}", matching.len(), all.len());
}
