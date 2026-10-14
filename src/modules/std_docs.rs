//! The standard library's index (card std-module-docs, notes/stdlib.md §8 "Discoverability"), read from lib/*.warp:
//! a module's leading comment says what it is, the comment right above a definition (its wrapped lines joined) what
//! that word does. `warp help list` prints one module, `warp help --markdown` the page wiki/standard-library.md, so
//! docs never drift from lib/.

const COMMENT: &str = "//";
const DEFINE: &str = ":=";
const ASSIGN: char = '=';
const CLASS: &str = "class ";
/// `global tempo = 120`: a module's variable, documented as `tempo`
const GLOBAL: &str = "global ";
const BLOCK_START: char = '{';
/// lib/*.warp wraps its comments by hand, near 120 columns: a comment line at least this long that ends no sentence goes
/// on in the next one; the stacked comments that are not one (`// attributes that are present or absent`) are short
const WRAPPED_LINE_LENGTH: usize = 90;
const SENTENCE_END: char = '.';
const MARKDOWN_TITLE: &str = "# Standard library";
const MARKDOWN_INTRO: &str = "Made by `warp help --markdown` from lib/*.warp: each module with its words, a word with the comment above it. A module comes with `use <module>`; the prelude needs none.";

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
	// the comment right above, the lines of its paragraph
	let mut comment: Vec<String> = vec![];
	let mut previous_comment_line: Option<&str> = None;
	for line in lines {
		if let Some(text) = comment_text(line) {
			if !previous_comment_line.is_some_and(is_wrapped) {
				comment.clear();
			}
			comment.push(text);
			previous_comment_line = Some(line);
			continue;
		}
		if let Some(signature) =
			signature(line).filter(|signature| defined.iter().any(|word| word == word_of(signature)))
		{
			words.push(Word { signature, doc: (!comment.is_empty()).then(|| comment.join(" ")) });
		}
		comment.clear();
		previous_comment_line = None;
	}
	(summary.join(" "), words)
}

fn comment_text(line: &str) -> Option<String> {
	line.trim_start().strip_prefix(COMMENT).map(|text| text.trim().to_string())
}

/// Whether a comment line goes on in the next one: it is long and ends no sentence
fn is_wrapped(line: &str) -> bool {
	let line = line.trim_end();
	line.chars().count() >= WRAPPED_LINE_LENGTH && !line.ends_with(SENTENCE_END)
}

/// A definition at the start of a line: `zip(a, b) := …` → `zip(a, b)`, `class Stack { …` → `class Stack`,
/// `html_attributes = […]` and `global html_attributes = […]` → `html_attributes`, `def comb(a) {` → `comb(a)`
fn signature(line: &str) -> Option<String> {
	if !line.starts_with(|first: char| first.is_alphabetic() || first == '_') {
		return None;
	}
	let line = line.strip_prefix(GLOBAL).unwrap_or(line);
	// `def hann_weighted(window) {`
	let keyword_definition =
		crate::operators::FUNCTION_KEYWORDS.iter().find_map(|keyword| line.strip_prefix(keyword)?.strip_prefix(' '));
	if let Some(definition) = keyword_definition {
		let head = definition.split(BLOCK_START).next().unwrap_or(definition);
		return Some(head.split(DEFINE).next().unwrap_or(head).trim().to_string());
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

/// A standard module word's head as its module writes it: `replace` of the prelude → `replace(t, old, new)`
pub fn word_head(module: &str, word: &str) -> Option<String> {
	module_index(module).1.into_iter().map(|entry| entry.signature).find(|signature| word_of(signature) == word)
}

/// `help(midi)`: each head of a standard module word with the comment above it, one definition after the other
pub fn word_help(module: &str, word: &str) -> Option<String> {
	let entries: Vec<String> = module_index(module)
		.1
		.into_iter()
		.filter(|entry| word_of(&entry.signature) == word)
		.map(|entry| match entry.doc {
			Some(doc) => format!("{}\n{doc}", entry.signature),
			None => entry.signature,
		})
		.collect();
	(!entries.is_empty()).then(|| entries.join("\n"))
}

/// `function(beep).dump` of a standard module word: its definitions as written, each with the indented lines of its
/// body (`tone(pitch, duration) := …`, `global tempo = 120`)
pub fn word_definition(module: &str, word: &str) -> Option<String> {
	let source = crate::modules::std_module_source(module)?;
	let mut definitions: Vec<String> = vec![];
	let mut inside = false;
	for line in source.lines() {
		let continues = line.starts_with([' ', '\t', '}', ')', ']']);
		if inside && continues {
			definitions.last_mut()?.push_str(&format!("\n{line}"));
			continue;
		}
		inside = signature(line).is_some_and(|signature| word_of(&signature) == word);
		if inside {
			definitions.push(line.to_string());
		}
	}
	(!definitions.is_empty()).then(|| definitions.join("\n"))
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
