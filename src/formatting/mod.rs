//! `warp format` (= `warp lint`): the one canonical layout of warp code (decision warp-format, notes/warp_format.md).
//! Only layout changes (indentation, spaces, blank lines), and only where the parsed tree stays the same (for beautify:
//! or the lowered program): an edit that
//! would change the meaning (`f it-1` spaced, an offside block re-indented) is left out, never applied.

mod lines;
mod rules;
mod soft_hints;
mod synonyms;

use crate::compiler::meta::LineInfo;
use crate::node::{Node, COMMENT_KEY};
use crate::warp_parser::{SpacedMinus, WarpParser};
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

/// The candidate edits that keep the meaning, on top of the kept ones: all at once when they can, else each half
/// on its own, down to the single edit that changes the meaning
fn keeping_the_meaning(
	lines: &[Line],
	kept: Vec<Edit>,
	candidates: &[Edit],
	keeps: &dyn Fn(&str, &[usize]) -> bool,
) -> Vec<Edit> {
	if candidates.is_empty() {
		return kept;
	}
	let trying: Vec<Edit> = kept.iter().chain(candidates).cloned().collect();
	let result = edited(lines, &trying);
	let edited_numbers: Vec<usize> = trying.iter().map(|(index, _)| lines[*index].number).collect();
	let changed: Vec<usize> =
		(1..=result.len()).filter(|line| edited_numbers.contains(&result[line - 1].number)).collect();
	if keeps(&joined(&result), &changed) {
		return trying;
	}
	if candidates.len() == 1 {
		return kept;
	}
	let (first, second) = candidates.split_at(candidates.len() / 2);
	let kept = keeping_the_meaning(lines, kept, first, keeps);
	keeping_the_meaning(lines, kept, second, keeps)
}

/// The program as the emitter gets it, what its warnings, assumptions and hints say, and the lines of its assumptions;
/// None when it does not lower
struct Lowered {
	program: String,
	messages: Vec<String>,
	assumed_lines: Vec<usize>,
}

pub(super) fn lowered(code: &str) -> Option<String> {
	crate::pipeline::lower(code).ok().map(|program| shape(&without_spaced_minus(program)))
}

/// The lowered program without the parser's mark of a minus written apart: lowering read it already (`f - 2`), so it
/// is layout only there
fn without_spaced_minus(node: Node) -> Node {
	match node {
		Node::Meta { node, data } if matches!(data.as_ref(), Node::Data(value) if value.downcast_ref::<SpacedMinus>().is_some()) => {
			without_spaced_minus(*node)
		}
		other => crate::node::children_rewritten(other, without_spaced_minus),
	}
}

fn lowered_with_messages(code: &str) -> Option<Lowered> {
	crate::errors::diagnostic::take_warnings(); // only this code's warnings
	let (program, hints) = crate::errors::diagnostic::every_hint_of(|| lowered(code));
	// an Ask's default is recorded even while quiet: `f it - 1` assumes f(it - 1)
	let assumptions = crate::errors::diagnostic::take_assumptions();
	let assumed_lines = assumptions.iter().map(|assumption| assumption.line).collect();
	let said = crate::errors::diagnostic::take_warnings().into_iter().chain(assumptions);
	let messages = said.chain(hints).map(|diagnostic| diagnostic.message).collect();
	Some(Lowered { program: program?, messages, assumed_lines })
}

/// beautify's second chance for an edit that changes the parsed tree (card format-beautify): `o-b` of two variables
/// parses as one name and lowers to the subtraction `o - b` does, so the spacing keeps the program; an edit that
/// lowers the same but makes the compiler say something new stays out, and so does one on a line that carries an
/// assumption (reviewer): `print f it-1` spaced reads as f(it) - 1, where the program takes f(it - 1)
fn lowers_the_same(original: &Lowered, candidate: &str, changed_lines: &[usize]) -> bool {
	lowered_with_messages(candidate).is_some_and(|candidate| {
		candidate.program == original.program
			&& !candidate.assumed_lines.iter().any(|line| changed_lines.contains(line))
			&& candidate.messages.iter().all(|message| original.messages.contains(message))
	})
}

/// The canonical layout of the code; Err when the code does not parse (there is no meaning to keep)
pub fn format(code: &str, options: Options) -> Result<Formatted, String> {
	formatted_after(code, Vec::new(), options, false)
}

/// `warp beautify`: another language's words become the warp words (synonyms.rs), then the layout with hash-comments
pub fn beautify(code: &str) -> Result<Formatted, String> {
	let (code, changes) = synonyms::canonical_synonyms(code);
	formatted_after(&code, changes, Options { hash_comments: true }, true)
}

/// The layout of `code`, after the `changes` made to it already; `by_lowering`: an edit that changes the parsed tree is
/// kept when the program lowers the same
fn formatted_after(
	code: &str,
	mut changes: Vec<Change>,
	options: Options,
	by_lowering: bool,
) -> Result<Formatted, String> {
	crate::errors::diagnostic::quietly(|| {
		let original = parsed(code);
		if has_error(&original) {
			return Err("it does not parse".to_string());
		}
		let tree = shape(&original);
		let original_lowered = by_lowering.then(|| lowered_with_messages(code)).flatten();
		let keeps = |text: &str, changed_lines: &[usize]| {
			shape(&parsed(text)) == tree
				|| original_lowered.as_ref().is_some_and(|original| lowers_the_same(original, text, changed_lines))
		};
		let mut lines: Vec<Line> =
			code.lines().enumerate().map(|(index, text)| Line { number: index + 1, text: text.to_string() }).collect();
		for rule in RULES.iter().filter(|rule| options.hash_comments || !rule.hash_comments_only) {
			let texts: Vec<String> = lines.iter().map(|line| line.text.clone()).collect();
			let candidates = (rule.edits)(&texts, &lines::scan(&texts));
			let kept = keeping_the_meaning(&lines, Vec::new(), &candidates, &keeps);
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
