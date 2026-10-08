//! Every word of the standard library (lib/*.warp) is called by some test (card std-word, notes/stdlib.md "Coverage").
//! A word counts as covered when a test names it or the body of a covered word calls it; src/ doesn't count (its
//! alias tables name words no program calls).
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const DEFINITION_KEYWORDS: [&str; 5] = ["def ", "fun ", "class ", "type ", "global "];
const DEFINITION_MARKS: [&str; 4] = ["(", ":=", "=", "{"];
const MENTIONING_EXTENSIONS: [&str; 2] = ["rs", "warp"];

fn files_below(folder: &Path, extensions: &[&str]) -> Vec<PathBuf> {
	let mut files = vec![];
	for entry in std::fs::read_dir(folder).unwrap().flatten() {
		let path = entry.path();
		if path.is_dir() {
			files.extend(files_below(&path, extensions));
		} else if path.extension().is_some_and(|extension| extensions.contains(&extension.to_str().unwrap())) {
			files.push(path);
		}
	}
	files
}

fn identifiers(text: &str) -> BTreeSet<&str> {
	text.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|word| !word.is_empty()).collect()
}

/// `name(…) := …`, `name = …`, `class Name {…`: a top-level line naming a word
fn defined_word(line: &str) -> Option<&str> {
	let mut rest = line;
	for keyword in DEFINITION_KEYWORDS {
		rest = rest.strip_prefix(keyword).unwrap_or(rest);
	}
	let name_length = rest.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(rest.len());
	let (name, after) = rest.split_at(name_length);
	let starts_like_a_name = name.starts_with(|c: char| c.is_alphabetic() || c == '_');
	let marked = DEFINITION_MARKS.iter().any(|mark| after.trim_start().starts_with(mark));
	(starts_like_a_name && marked).then_some(name)
}

/// word → its definition text (up to the next top-level definition), over all lib/*.warp
fn library_words(lib: &Path) -> BTreeMap<String, String> {
	let mut words = BTreeMap::new();
	for file in files_below(lib, &["warp"]).iter().filter(|file| file.parent() == Some(lib)) {
		let mut current: Option<String> = None;
		for line in std::fs::read_to_string(file).unwrap().lines() {
			if let Some(name) = defined_word(line) {
				current = Some(name.to_string());
			}
			if let Some(name) = &current {
				let body: &mut String = words.entry(name.clone()).or_default();
				body.push_str(line);
				body.push('\n');
			}
		}
	}
	words
}

fn uncovered_library_words(root: &Path) -> Vec<String> {
	let words = library_words(&root.join("lib"));
	let mentioning_text: String = files_below(&root.join("tests"), &MENTIONING_EXTENSIONS)
		.into_iter()
		.map(|file| std::fs::read_to_string(file).unwrap())
		.collect::<Vec<_>>()
		.join("\n");
	let mentioned = identifiers(&mentioning_text);
	let mut covered: BTreeSet<&str> = words.keys().map(String::as_str).filter(|word| mentioned.contains(word)).collect();
	let mut pending: Vec<&str> = covered.iter().copied().collect();
	while let Some(word) = pending.pop() {
		for called in identifiers(&words[word]) {
			if words.contains_key(called) && covered.insert(called) {
				pending.push(called);
			}
		}
	}
	words.keys().filter(|word| !covered.contains(word.as_str())).cloned().collect()
}

#[test]
fn every_library_word_is_called_by_a_test() {
	let uncovered = uncovered_library_words(Path::new(env!("CARGO_MANIFEST_DIR")));
	assert!(uncovered.is_empty(), "lib/*.warp words no test calls: {}", uncovered.join(" "));
}
