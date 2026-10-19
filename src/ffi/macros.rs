//! The C preprocessor as far as declarations need it (card wasmtime-ffi): wasm.h declares its functions and types
//! through macros (`WASM_DECLARE_OWN(engine)` declares `wasm_engine_t` and `wasm_engine_delete`) and marks them with
//! words defined empty (`WASM_API_EXTERN`, `own`). A header's text comes back with comments and line continuations
//! gone, the invocations of function-like macros expanded (`##` pasted), one declaration per line, and words some
//! branch defines empty dropped. Conditionals are not evaluated: every branch is read, an empty definition in any of
//! them wins, except for C's own words (`#define const` in K&R branches of sys/cdefs.h). Macros a header defines serve the
//! headers of the same library read after it (its includes come first)

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

/// Nested expansions deeper than this are left as written (a macro expanding to itself)
const MAX_EXPANSION_DEPTH: usize = 8;
const CONTINUATION: &str = "\\\n";
const DEFINE: &str = "#define";
const PASTE: &str = "##";
/// C's words a branch for old compilers defines empty: never dropped
const C_KEYWORDS: [&str; 6] = ["const", "volatile", "signed", "inline", "restrict", "void"];

#[derive(Default)]
struct Macros {
	/// `#define WASM_DECLARE_OWN(name) …`: parameters and body
	functions: HashMap<String, (Vec<String>, String)>,
	/// `#define own`: dropped wherever it stands
	empty: HashSet<String>,
}

/// The macros of each library read so far
static MACROS: Mutex<Option<HashMap<String, Macros>>> = Mutex::new(None);

/// `content`, a header of `library`, as the declaration parser reads it (see the module comment)
pub(super) fn preprocessed(content: &str, library: &str) -> String {
	let mut in_comment = false;
	let uncommented: Vec<String> = content
		.lines()
		.map(|line| super::header::without_block_comments(line, &mut in_comment))
		.map(|line| match line.find("//") {
			Some(start) => line[..start].to_string(),
			None => line,
		})
		.collect();
	let joined = uncommented.join("\n").replace(CONTINUATION, " ");
	let mut guard = MACROS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let macros = guard.get_or_insert_with(Default::default).entry(library.to_string()).or_default();
	let mut lines = vec![];
	for line in joined.lines() {
		let trimmed = line.trim();
		if let Some(definition) = trimmed.strip_prefix(DEFINE) {
			macros.define(definition.trim());
			continue;
		}
		if trimmed.starts_with('#') {
			continue;
		}
		let expanded = macros.expanded(trimmed, 0);
		lines.extend(one_declaration_per_line(&macros.without_empty_words(&expanded)));
	}
	lines.join("\n")
}

impl Macros {
	fn define(&mut self, definition: &str) {
		let name_end = definition.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(definition.len());
		let (name, rest) = definition.split_at(name_end);
		if name.is_empty() || C_KEYWORDS.contains(&name) {
			return;
		}
		match rest.strip_prefix('(').and_then(|rest| rest.split_once(')')) {
			Some((parameters, body)) => {
				let parameters = parameters.split(',').map(|parameter| parameter.trim().to_string()).collect();
				self.functions.insert(name.to_string(), (parameters, body.trim().to_string()));
			}
			None if rest.trim().is_empty() => {
				self.empty.insert(name.to_string());
			}
			None => {}
		}
	}

	/// `text` with each invocation of a function-like macro replaced by its body, its arguments put in
	fn expanded(&self, text: &str, depth: usize) -> String {
		if depth > MAX_EXPANSION_DEPTH || self.functions.is_empty() {
			return text.to_string();
		}
		let mut result = String::new();
		let mut rest = text;
		while let Some((start, name)) = next_word(rest) {
			let after = &rest[start + name.len()..];
			let invocation = self.functions.get(name).filter(|_| after.trim_start().starts_with('('));
			let Some((parameters, body)) = invocation else {
				result.push_str(&rest[..start + name.len()]);
				rest = after;
				continue;
			};
			let open = after.find('(').unwrap_or_default();
			let Some(close) = super::header::matching_paren(after, open) else {
				result.push_str(&rest[..start + name.len()]);
				rest = after;
				continue;
			};
			let arguments: Vec<&str> = split_arguments(&after[open + 1..close]);
			result.push_str(&rest[..start]);
			result.push_str(&self.expanded(&substituted(body, parameters, &arguments), depth + 1));
			rest = &after[close + 1..];
		}
		result.push_str(rest);
		result
	}

	fn without_empty_words(&self, text: &str) -> String {
		let mut result = String::new();
		let mut rest = text;
		while let Some((start, word)) = next_word(rest) {
			result.push_str(&rest[..start]);
			if !self.empty.contains(word) {
				result.push_str(word);
			}
			rest = &rest[start + word.len()..];
		}
		result.push_str(rest);
		result
	}
}

/// The next identifier in `text`: where it starts, and it
fn next_word(text: &str) -> Option<(usize, &str)> {
	let is_word_letter = |c: char| c.is_ascii_alphanumeric() || c == '_';
	let mut position = 0;
	while position < text.len() {
		let rest = &text[position..];
		let start = position + rest.find(|c: char| c.is_ascii_alphabetic() || c == '_')?;
		// a word starting inside a number (`10u`) or after a letter is no identifier start
		let preceded_by_word = text[..start].chars().next_back().is_some_and(is_word_letter);
		let end = text[start..].find(|c: char| !is_word_letter(c)).map_or(text.len(), |length| start + length);
		if !preceded_by_word {
			return Some((start, &text[start..end]));
		}
		position = end;
	}
	None
}

/// The arguments of an invocation (or a declaration's parameters), split at the commas outside parentheses
pub(super) fn split_arguments(text: &str) -> Vec<&str> {
	let mut arguments = vec![];
	let (mut depth, mut start) = (0, 0);
	for (index, c) in text.char_indices() {
		match c {
			'(' => depth += 1,
			')' => depth -= 1,
			',' if depth == 0 => {
				arguments.push(text[start..index].trim());
				start = index + 1;
			}
			_ => {}
		}
	}
	arguments.push(text[start..].trim());
	arguments
}

/// `body` with each parameter replaced by its argument, then `a ## b` pasted to `ab`
fn substituted(body: &str, parameters: &[String], arguments: &[&str]) -> String {
	let mut result = String::new();
	let mut rest = body;
	while let Some((start, word)) = next_word(rest) {
		result.push_str(&rest[..start]);
		match parameters.iter().position(|parameter| parameter == word) {
			Some(index) => result.push_str(arguments.get(index).copied().unwrap_or_default()),
			None => result.push_str(word),
		}
		rest = &rest[start + word.len()..];
	}
	result.push_str(rest);
	let mut pasted = result;
	while let Some(at) = pasted.find(PASTE) {
		let before = pasted[..at].trim_end().len();
		let after =
			at + PASTE.len() + (pasted[at + PASTE.len()..].len() - pasted[at + PASTE.len()..].trim_start().len());
		pasted.replace_range(before..after, "");
	}
	pasted
}

/// An expanded line split after each `;` and `{` outside parentheses, so each declaration (and each field of a struct
/// a macro defines) is a line of its own, as a header writes them
fn one_declaration_per_line(text: &str) -> Vec<String> {
	let mut lines = vec![];
	let mut current = String::new();
	let mut depth = 0;
	for c in text.chars() {
		current.push(c);
		match c {
			'(' => depth += 1,
			')' => depth -= 1,
			';' | '{' if depth == 0 => lines.push(std::mem::take(&mut current).trim().to_string()),
			_ => {}
		}
	}
	lines.push(current.trim().to_string());
	lines
}
