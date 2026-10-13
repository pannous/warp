//! Twins, checked (card self-differential, notes/self_hosting.md step 4): a word of a warp-written compiler part in
//! lib/compiler/ runs over every sample and probe next to its Rust twin, and the first file where the two differ fails
//! the test with both texts around the first difference.
use std::path::{Path, PathBuf};
use warp::wasm_emitter::eval;
use warp::{Bracket, Node, Separator};

const SOURCE_FOLDERS: [&str; 2] = ["samples", "probes"];
const SOURCE_EXTENSION: &str = "warp";
const COMPILER_FOLDER: &str = "lib/compiler";
/// Characters shown on each side of the first difference
const CONTEXT: usize = 40;
/// Sources per evaluated program: each program's store is freed before the next, which keeps the test under 1 GB
const SOURCES_PER_PROGRAM: usize = 64;

/// Every sample and probe source, sorted; a file that is not UTF-8 text is no source
pub fn sources() -> Vec<PathBuf> {
	let mut found = Vec::new();
	for folder in SOURCE_FOLDERS {
		collect_sources(Path::new(folder), &mut found);
	}
	found.retain(|path| std::fs::read_to_string(path).is_ok());
	found.sort();
	found
}

fn collect_sources(folder: &Path, found: &mut Vec<PathBuf>) {
	for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
		let path = entry.path();
		if path.is_dir() {
			collect_sources(&path, found);
		} else if path.extension().is_some_and(|extension| extension == SOURCE_EXTENSION) {
			found.push(path);
		}
	}
}

/// A Rust twin's list as warp writes it: the empty list is ø
pub fn warp_list(items: impl Iterator<Item = Node>) -> Node {
	let items: Vec<Node> = items.collect();
	if items.is_empty() {
		return Node::Empty;
	}
	Node::List(items, Bracket::Square, Separator::None)
}

/// `word(read(path))` of lib/compiler/<part>.warp for every source, each written as the Rust twin's value of the same
/// text is written
pub fn differential(part: &str, word: &str, rust_twin: impl Fn(&str) -> Node) {
	for paths in sources().chunks(SOURCES_PER_PROGRAM) {
		for (path, value) in paths.iter().zip(warp_values(part, word, paths)) {
			let text = std::fs::read_to_string(path).unwrap();
			let (warp_text, rust_text) = (value.serialize(), rust_twin(&text).serialize());
			if warp_text != rust_text {
				panic!(
					"{word} differs from its Rust twin on {}:\n  warp: …{}…\n  rust: …{}…",
					path.display(),
					around_difference(&warp_text, &rust_text),
					around_difference(&rust_text, &warp_text)
				);
			}
		}
	}
}

/// `word(read(path))` for the sources, in one program
fn warp_values(part: &str, word: &str, paths: &[PathBuf]) -> Vec<Node> {
	let quoted: Vec<String> = paths.iter().map(|path| format!("\"{}\"", path.display())).collect();
	let program = format!(
		"use \"{COMPILER_FOLDER}/{part}.warp\"\n[{word}(read(path)) for path in [{}]]",
		quoted.join(", ")
	);
	let result = eval(&program);
	let Node::List(values, ..) = result.drop_meta().clone() else {
		panic!("the warp twin {word} gave no list of values: {}", result.serialize())
	};
	assert_eq!(
		values.len(),
		paths.len(),
		"the warp twin {word} gave a value for each source"
	);
	values
}

/// The text around its first character that differs from the other text
fn around_difference(text: &str, other: &str) -> String {
	let chars: Vec<char> = text.chars().collect();
	let first = chars.iter().zip(other.chars()).take_while(|(a, b)| *a == b).count();
	let start = first.saturating_sub(CONTEXT);
	chars[start..(first + CONTEXT).min(chars.len())].iter().collect()
}
