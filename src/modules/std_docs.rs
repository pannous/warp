//! The standard library's index (card std-module-docs, notes/stdlib.md §8 "Discoverability"), read from lib/*.warp:
//! a module's leading comment says what it is, the comment block right above a definition (its lines joined) what
//! that word does; a blank line ends a block, so a section note stands apart. `warp help list` prints one module, `warp help --markdown` the page wiki/standard-library.md, so
//! docs never drift from lib/.

const COMMENT: &str = "//";
const DEFINE: &str = ":=";
const ASSIGN: char = '=';
const CLASS: &str = "class ";
/// `global tempo = 120`: a module's variable, documented as `tempo`
const GLOBAL: &str = "global ";
const BLOCK_START: char = '{';
const MARKDOWN_TITLE: &str = "# Standard library";
const MARKDOWN_INTRO: &str = "Made by `warp help --markdown` from lib/*.warp: each module with its words, a word with the comment above it. A module comes with `use <module>`; the prelude needs none.";

/// A word of a module: how it is written (`zip(a, b)`, `class Stack`) and its comment line, if any
struct Word {
	signature: String,
	doc: Option<String>,
}

/// A module's leading comment, joined, and its words in source order
type ModuleIndex = (String, Vec<Word>);

thread_local! {
	/// Each module's index by its name and source (a checkout's lib/ gives another): the library signatures read a
	/// prelude word's head from it for each word a program names (library_signatures.rs)
	static INDEXES: std::cell::RefCell<std::collections::HashMap<(String, usize, usize), std::rc::Rc<ModuleIndex>>> =
		Default::default();
}

/// The module's leading comment and its words, indexed once per source
fn module_index(module: &str) -> std::rc::Rc<ModuleIndex> {
	let source = crate::modules::std_module_source(module).unwrap_or_default();
	let key = (module.to_string(), source.as_ptr() as usize, source.len());
	if let Some(index) = INDEXES.with_borrow(|indexes| indexes.get(&key).cloned()) {
		return index;
	}
	let index = std::rc::Rc::new(indexed_module(module, source));
	INDEXES.with_borrow_mut(|indexes| indexes.insert(key, index.clone()));
	index
}

/// The module's leading comment, joined, and its words (those it defines, std_module_definitions) in source order
fn indexed_module(module: &str, source: &str) -> ModuleIndex {
	let defined = crate::modules::std_module_definitions(module).unwrap_or_default();
	let mut lines = source.lines().peekable();
	let mut summary = vec![];
	while let Some(text) = lines.peek().and_then(|line| comment_text(line)) {
		summary.push(text);
		lines.next();
	}
	let mut words = vec![];
	// the comment lines right above, without a blank line between them and the definition
	let mut comment: Vec<String> = vec![];
	for line in lines {
		if let Some(text) = comment_text(line) {
			comment.push(text);
			continue;
		}
		if let Some(signature) =
			signature(line).filter(|signature| defined.iter().any(|word| word == word_of(signature)))
		{
			words.push(Word { signature, doc: (!comment.is_empty()).then(|| comment.join(" ")) });
		}
		comment.clear();
	}
	(summary.join(" "), words)
}

fn comment_text(line: &str) -> Option<String> {
	line.trim_start().strip_prefix(COMMENT).map(|text| text.trim().to_string())
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
	module_index(module).1.iter().map(|entry| &entry.signature).find(|signature| word_of(signature) == word).cloned()
}

/// `help(midi)`: each head of a standard module word with the comment above it, one definition after the other
pub fn word_help(module: &str, word: &str) -> Option<String> {
	let entries: Vec<String> = module_index(module)
		.1
		.iter()
		.filter(|entry| word_of(&entry.signature) == word)
		.map(|entry| match &entry.doc {
			Some(doc) => format!("{}\n{doc}", entry.signature),
			None => entry.signature.clone(),
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
	let index = module_index(module);
	let (summary, words) = (&index.0, &index.1);
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
		let index = module_index(module);
		let (summary, words) = (&index.0, &index.1);
		page.extend([String::new(), format!("## {module}"), String::new(), summary.clone(), String::new()]);
		page.extend(words.iter().map(|word| match &word.doc {
			Some(doc) => format!("- `{}`: {doc}", word.signature),
			None => format!("- `{}`", word.signature),
		}));
	}
	page.push(String::new());
	page.join("\n")
}
