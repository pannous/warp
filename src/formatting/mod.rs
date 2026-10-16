//! `warp format` (= `warp lint`): the one canonical layout of warp code (decision warp-format, notes/warp_format.md).
//! Only layout changes (indentation, spaces, blank lines), and only where the parsed tree stays the same: an edit that
//! would change the meaning (`f it-1` spaced, an offside block re-indented) is left out, never applied.

mod lines;
mod rules;
mod soft_hints;
mod synonyms;

use crate::compiler::meta::LineInfo;
use crate::node::{Node, COMMENT_KEY};
use crate::warp_parser::WarpParser;
pub(crate) use lines::code_characters;
use rules::{Edit, RULES};
pub use soft_hints::{soft_hints, Hint};

pub struct Change {
	/// the line in the source as given, from 1
	pub line: usize,
	pub rule: &'static str,
}

pub struct Formatted {
	pub text: String,
	pub changes: Vec<Change>,
}

#[derive(Default, Clone, Copy)]
pub struct Options {
	/// `// text` → `# text` (`warp format hash-comments`)
	pub hash_comments: bool,
}

#[derive(Clone)]
struct Line {
	number: usize,
	text: String,
}

/// What a Meta layer adds to the meaning: nothing for a position or a comment; the rest (`@unit(cm)`, a spaced minus,
/// a literal's source text) is kept
fn meta_meaning(data: &Node) -> Option<String> {
	match data {
		Node::Data(value) if value.downcast_ref::<LineInfo>().is_some() => None,
		Node::Data(value) => Some(value.type_name.clone()),
		Node::Key(key, _, _) if matches!(key.as_ref(), Node::Symbol(name) if name == COMMENT_KEY) => None,
		other => Some(shape(other)),
	}
}

/// The tree without positions and comments, as a text to compare: the meaning the layout must keep
fn shape(node: &Node) -> String {
	match node {
		Node::Meta { node, data } => match meta_meaning(data) {
			Some(meaning) => format!("meta({meaning} {})", shape(node)),
			None => shape(node),
		},
		Node::Key(key, op, value) => format!("({} {op:?} {})", shape(key), shape(value)),
		Node::List(items, bracket, separator) => {
			let items: Vec<String> = items.iter().map(shape).collect();
			format!("{bracket}{separator:?} {}{}", items.join(" "), bracket.closing())
		}
		Node::Type { name, body } => format!("type({} {})", shape(name), shape(body)),
		Node::Error(inner) => format!("error({})", shape(inner)),
		other => format!("{other:?}"),
	}
}

fn has_error(node: &Node) -> bool {
	match node {
		Node::Error(_) => true,
		Node::Meta { node, .. } => has_error(node),
		Node::Key(key, _, value) => has_error(key) || has_error(value),
		Node::List(items, _, _) => items.iter().any(has_error),
		Node::Type { name, body } => has_error(name) || has_error(body),
		_ => false,
	}
}

fn parsed(text: &str) -> Node {
	WarpParser::parse(text)
}

fn joined(lines: &[Line]) -> String {
	lines.iter().map(|line| format!("{}\n", line.text)).collect()
}

/// The lines with these edits, a deleted line gone
fn edited(lines: &[Line], edits: &[Edit]) -> Vec<Line> {
	let mut result = lines.to_vec();
	let mut deleted = vec![false; lines.len()];
	for (index, replacement) in edits {
		match replacement {
			Some(text) => result[*index].text = text.clone(),
			None => deleted[*index] = true,
		}
	}
	result.into_iter().zip(deleted).filter(|(_, deleted)| !deleted).map(|(line, _)| line).collect()
}

/// The candidate edits that keep the tree, on top of the kept ones: all at once when they can, else each half
/// on its own, down to the single edit that changes the meaning
fn keeping_the_tree(lines: &[Line], kept: Vec<Edit>, candidates: &[Edit], tree: &str) -> Vec<Edit> {
	if candidates.is_empty() {
		return kept;
	}
	let trying: Vec<Edit> = kept.iter().chain(candidates).cloned().collect();
	if shape(&parsed(&joined(&edited(lines, &trying)))) == tree {
		return trying;
	}
	if candidates.len() == 1 {
		return kept;
	}
	let (first, second) = candidates.split_at(candidates.len() / 2);
	let kept = keeping_the_tree(lines, kept, first, tree);
	keeping_the_tree(lines, kept, second, tree)
}

/// The canonical layout of the code; Err when the code does not parse (there is no meaning to keep)
pub fn format(code: &str, options: Options) -> Result<Formatted, String> {
	formatted_after(code, Vec::new(), options)
}

/// `warp beautify`: another language's words become the warp words (synonyms.rs), then the layout with hash-comments
pub fn beautify(code: &str) -> Result<Formatted, String> {
	let (code, changes) = synonyms::canonical_synonyms(code);
	formatted_after(&code, changes, Options { hash_comments: true })
}

/// The layout of `code`, after the `changes` made to it already
fn formatted_after(code: &str, mut changes: Vec<Change>, options: Options) -> Result<Formatted, String> {
	crate::errors::diagnostic::quietly(|| {
		let original = parsed(code);
		if has_error(&original) {
			return Err("it does not parse".to_string());
		}
		let tree = shape(&original);
		let mut lines: Vec<Line> =
			code.lines().enumerate().map(|(index, text)| Line { number: index + 1, text: text.to_string() }).collect();
		for rule in RULES.iter().filter(|rule| options.hash_comments || !rule.hash_comments_only) {
			let texts: Vec<String> = lines.iter().map(|line| line.text.clone()).collect();
			let candidates = (rule.edits)(&texts, &lines::scan(&texts));
			let kept = keeping_the_tree(&lines, Vec::new(), &candidates, &tree);
			let mut numbers: Vec<usize> = kept.iter().map(|(index, _)| lines[*index].number).collect();
			numbers.sort();
			changes.extend(numbers.into_iter().map(|line| Change { line, rule: rule.name }));
			lines = edited(&lines, &kept);
		}
		let text = joined(&lines);
		if !code.ends_with('\n') && !code.is_empty() {
			changes.push(Change { line: code.lines().count(), rule: "final newline" });
		}
		Ok(Formatted { text, changes })
	})
}
