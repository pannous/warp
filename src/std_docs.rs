//! The standard library's index (card std-module-docs, notes/stdlib.md §8 "Discoverability"), read from lib/*.warp:
//! a module's leading comment says what it is, the comment line right above a definition what that word does.
//! `warp help list` prints one module, `warp help --markdown` the page wiki/standard-library.md, so docs never drift
//! from lib/.

const COMMENT: &str = "//";
const DEFINE: &str = ":=";
const ASSIGN: char = '=';
const CLASS: &str = "class ";
const BLOCK_START: char = '{';
const MARKDOWN_TITLE: &str = "# Standard library";
const MARKDOWN_INTRO: &str = "Made by `warp help --markdown` from lib/*.warp: each module with its words, a word with the comment line above it. A module comes with `use <module>`; the prelude needs none.";

/// A word of a module: how it is written (`zip(a, b)`, `class Stack`) and its comment line, if any
struct Word {
	signature: String,
	doc: Option<String>,
}

/// The module's leading comment, joined, and its words (those it defines, std_module_definitions) in source order
fn module_index(module: &str) -> (String, Vec<Word>) {
	let source = crate::modules::std_module_source(module).unwrap_or_default();
	let defined = crate::modules::std_module_definitions(module).unwrap_or_default();
	let mut lines = source.lines().peekable();
	let mut summary = vec![];
	while let Some(text) = lines.peek().and_then(|line| comment_text(line)) {
		summary.push(text);
		lines.next();
	}
	let mut words = vec![];
	let mut comment: Option<String> = None;
	for line in lines {
		if let Some(text) = comment_text(line) {
			comment.get_or_insert(text);
			continue;
		}
		if let Some(signature) = signature(line).filter(|signature| defined.iter().any(|word| word == word_of(signature))) {
			words.push(Word { signature, doc: comment.take() });
		}
		comment = None;
	}
	(summary.join(" "), words)
}

fn comment_text(line: &str) -> Option<String> {
	line.trim_start().strip_prefix(COMMENT).map(|text| text.trim().to_string())
}

/// A definition at the start of a line: `zip(a, b) := …` → `zip(a, b)`, `class Stack { …` → `class Stack`,
/// `html_attributes = […]` → `html_attributes`
fn signature(line: &str) -> Option<String> {
	if !line.starts_with(|first: char| first.is_alphabetic() || first == '_') {
		return None;
	}
	if line.starts_with(CLASS) {
		return Some(line.split(BLOCK_START).next().unwrap_or(line).trim().to_string());
	}
	let head = match line.find(DEFINE) {
		Some(end) => &line[..end],
		None => line.split(ASSIGN).next().filter(|head| head.len() < line.len())?,
	};
	Some(head.trim().to_string())
}

/// `zip(a, b)` → `zip`, `class Stack` → `Stack`
fn word_of(signature: &str) -> &str {
	let name = signature.strip_prefix(CLASS).unwrap_or(signature);
	name.split(['(', ':', ' ']).next().unwrap_or(name)
}

/// `warp help <module>`: the module, how to use it, and its words with their comment lines
pub fn module_help(module: &str) -> Option<String> {
	crate::modules::std_module_source(module)?;
	let (summary, words) = module_index(module);
	let width = words.iter().map(|word| word.signature.chars().count()).max().unwrap_or(0);
	let lines = words.iter().map(|word| match &word.doc {
		Some(doc) => format!("  {:width$}  {doc}", word.signature),
		None => format!("  {}", word.signature),
	});
	Some(std::iter::once(format!("{module}: {summary}")).chain(lines).collect::<Vec<_>>().join("\n"))
}

/// `warp help`: the modules there are
pub fn modules_overview() -> String {
	let names: Vec<&str> = crate::modules::std_module_names().collect();
	format!("standard modules (warp help <module>): {}", names.join(" "))
}

/// wiki/standard-library.md
pub fn standard_library_markdown() -> String {
	let mut page = vec![MARKDOWN_TITLE.to_string(), String::new(), MARKDOWN_INTRO.to_string()];
	for module in crate::modules::std_module_names() {
		let (summary, words) = module_index(module);
		page.extend([String::new(), format!("## {module}"), String::new(), summary, String::new()]);
		page.extend(words.iter().map(|word| match &word.doc {
			Some(doc) => format!("- `{}`: {doc}", word.signature),
			None => format!("- `{}`", word.signature),
		}));
	}
	page.push(String::new());
	page.join("\n")
}
