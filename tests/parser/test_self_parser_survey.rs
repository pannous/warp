//! card self-parser: every source parser.warp reads otherwise than the Rust parser, with both trees around the first
//! difference, to sort the mismatches into forms to keep and forms to drop (notes/self_hosting_dropped.md)
use crate::common::twins::{around_difference, differences, node_view, sources};
use warp::WarpParser;

/// Each differing source and both trees around its first difference, written by parser_differences
const DIFFERENCES: &str = "data/self_parser/differences.txt";

/// `tests/queue.sh -- parser_differences --ignored`
#[test]
#[ignore]
fn parser_differences() {
	let rust_tree = |text: &str| node_view(&WarpParser::parse(text));
	let report: Vec<String> = differences(&sources(), "parser", "tree", rust_tree)
		.iter()
		.map(|(path, warp_text, rust_text)| {
			format!(
				"{path}\n  warp: …{}…\n  rust: …{}…",
				around_difference(warp_text, rust_text),
				around_difference(rust_text, warp_text)
			)
		})
		.collect();
	std::fs::create_dir_all(std::path::Path::new(DIFFERENCES).parent().unwrap()).unwrap();
	std::fs::write(DIFFERENCES, report.join("\n") + "\n").unwrap();
	println!("{} differing sources, listed in {DIFFERENCES}", report.len());
}
