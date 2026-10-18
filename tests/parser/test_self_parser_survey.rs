//! card self-parser: every source parser.warp reads otherwise than the Rust parser, with both trees around the first
//! difference, to sort the mismatches into forms to keep and forms to drop (notes/self_hosting_dropped.md)
use super::test_self_parser::rust_tree;
use crate::common::twins::{around_difference, differences, sources};

/// The sources needing a form parser.warp drops (notes/self_hosting_dropped.md): path, tab, form
const DROPPED: &str = "tests/parser/self_parser_dropped.txt";
/// Each differing source and both trees around its first difference, written by parser_differences
const DIFFERENCES: &str = "data/self_parser/differences.txt";

/// `tests/queue.sh -- parser_differences --ignored`
#[test]
#[ignore]
fn parser_differences() {
	let listed = std::fs::read_to_string(DROPPED).unwrap();
	let dropped: Vec<&str> = listed.lines().filter_map(|line| line.split('\t').next()).collect();
	let kept: Vec<_> = sources().into_iter().filter(|path| !dropped.contains(&path.to_str().unwrap())).collect();
	let report: Vec<String> = differences(&kept, "parser", "tree", rust_tree)
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
	println!("{} of {} kept sources differ, listed in {DIFFERENCES}", report.len(), kept.len());
}
