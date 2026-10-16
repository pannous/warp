//! The layout rules (decision warp-format): each turns lines into edits, which mod.rs keeps only where the parsed
//! tree stays the same

use super::lines::{code_text, leading_closers, LineScan};
use crate::operators::all_spellings;
use std::sync::LazyLock;

/// Operator spellings that stay as written: `:` has its own rule, `.` `::` `?.` reach into a value, ranges stay glued
/// (`1..3`), the rest are prefix or postfix (`!x`, `x++`, `#xs`) or glue a unit (`m·kg`). Every other spelling of the
/// operator table is spaced; mod.rs drops a spacing that changes the tree (a tag `<div>`, `7//2`)
const GLUED_SPELLINGS: [&str; 19] =
	[":", ".", "::", "?.", "..", "..<", "...", "…", "!", "¬", "++", "--", "#", "?", "~", "·", "⋅", "⌞", "⌟"];

/// The binary operators written with a space on each side, longest first: the symbol spellings of the operator table
/// (src/operators/spellings.rs), Unicode ones too (`≤`, `≠`, `×`)
static SPACED_OPERATORS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
	let mut spaced: Vec<&str> = all_spellings()
		.map(|(spelling, _)| spelling)
		.filter(|spelling| !spelling.chars().any(|c| c.is_alphanumeric() || c.is_whitespace()))
		.filter(|spelling| !GLUED_SPELLINGS.contains(spelling))
		.collect();
	spaced.sort_by_key(|spelling| (std::cmp::Reverse(spelling.chars().count()), *spelling));
	spaced.dedup();
	spaced
});
/// Characters of operators: an operator glued to one of these is part of another (`===`, `//`, `..`), left alone
static OPERATOR_CHARACTERS: LazyLock<String> = LazyLock::new(|| {
	all_spellings()
		.flat_map(|(spelling, _)| spelling.chars())
		.filter(|c| !c.is_alphanumeric() && !c.is_whitespace())
		.collect()
});
/// What ends an operand (before a binary operator) and what starts one (after it)
const OPERAND_ENDS: &str = "_)]}\"'`ø";
const OPERAND_STARTS: &str = "_([{\"'`#.$ø-!√";
/// A line ending in one of these continues on the next one, a line starting with one of these continues the one before;
/// `else` continues a `then` line (not a `}`)
const CONTINUING_ENDS: [&str; 9] = ["=", "+", "-", "*", "/", "|>", "&&", "||", "->"];
const CONTINUING_STARTS: [&str; 6] = ["|>", ".", "&&", "||", "? ", "then "];
const INDENT: &str = "\t";
/// Width of a tab when comparing the indentation as written
const TAB_WIDTH: usize = 4;

/// A line's new text, None to delete it
pub(super) type Edit = (usize, Option<String>);

pub(super) struct Rule {
	pub name: &'static str,
	pub edits: fn(&[String], &[LineScan]) -> Vec<Edit>,
	/// only with `warp format hash-comments`
	pub hash_comments_only: bool,
}

const fn rule(name: &'static str, edits: fn(&[String], &[LineScan]) -> Vec<Edit>) -> Rule {
	Rule { name, edits, hash_comments_only: false }
}

pub(super) const RULES: [Rule; 8] = [
	Rule { name: "# comment", edits: hash_comments, hash_comments_only: true },
	rule("trailing whitespace", trailing_whitespace),
	rule("blank lines", blank_lines),
	rule("spaces", spaces),
	rule("space after a comma or semicolon", separator_spacing),
	rule("space after a key's colon", colon_spacing),
	rule("spaces around an operator", operator_spacing),
	rule("indentation", indentation),
];

/// `// text` → `# text` (user, 2026-10-15: `//` reads like Python's floor division); `/// doc` → `## doc`; `//!` stays
fn hash_comments(lines: &[String], scans: &[LineScan]) -> Vec<Edit> {
	let mut edits = Vec::new();
	for (index, line) in lines.iter().enumerate() {
		let Some(at) = scans[index].comment else { continue };
		let comment = &line[at..];
		let (mark, text) = match comment.strip_prefix("///") {
			Some(text) => ("##", text),
			None => match comment.strip_prefix("//") {
				Some(text) if !text.starts_with('!') => ("#", text),
				_ => continue,
			},
		};
		let space = if text.is_empty() || text.starts_with([' ', '\t']) { "" } else { " " };
		edits.extend(changed(index, line, format!("{}{mark}{space}{text}", &line[..at])));
	}
	edits
}

fn changed(index: usize, line: &str, new: String) -> Option<Edit> {
	(new != line).then_some((index, Some(new)))
}

fn trailing_whitespace(lines: &[String], scans: &[LineScan]) -> Vec<Edit> {
	let trimmed = |(index, line): (usize, &String)| {
		(!scans[index].ends_in_text_or_comment()).then(|| changed(index, line, line.trim_end().to_string())).flatten()
	};
	lines.iter().enumerate().filter_map(trimmed).collect()
}

/// At most one blank line in a row, none at the start or the end
fn blank_lines(lines: &[String], scans: &[LineScan]) -> Vec<Edit> {
	let is_blank = |index: usize| lines[index].trim().is_empty() && !scans[index].starts_in_text_or_comment();
	let last_content = (0..lines.len()).rev().find(|&index| !is_blank(index));
	(0..lines.len())
		.filter(|&index| {
			is_blank(index) && (index == 0 || is_blank(index - 1) || last_content.is_none_or(|last| index > last))
		})
		.map(|index| (index, None))
		.collect()
}

/// The line's characters with whether each is code
fn characters(line: &str, scan: &LineScan) -> Vec<(char, bool)> {
	line.char_indices().map(|(at, c)| (c, scan.is_code(at))).collect()
}

fn indentation_length(characters: &[(char, bool)], scan: &LineScan) -> usize {
	if scan.starts_in_text_or_comment() {
		return 0;
	}
	characters.iter().take_while(|(c, _)| *c == ' ' || *c == '\t').count()
}

/// Rewrites the code characters of every line: `rewrite(characters, index, written)` pushes the replacement of the
/// character at `index` onto `written` and returns how many characters it took
fn rewrite_code(
	lines: &[String],
	scans: &[LineScan],
	rewrite: impl Fn(&[(char, bool)], usize, &mut String, &LineScan) -> usize,
) -> Vec<Edit> {
	let mut edits = Vec::new();
	for (index, line) in lines.iter().enumerate() {
		let characters = characters(line, &scans[index]);
		let indentation = indentation_length(&characters, &scans[index]);
		let mut written: String = characters[..indentation].iter().map(|(c, _)| c).collect();
		let mut at = indentation;
		while at < characters.len() {
			if characters[at].1 {
				at += rewrite(&characters, at, &mut written, &scans[index]).max(1);
			} else {
				written.push(characters[at].0);
				at += 1;
			}
		}
		edits.extend(changed(index, line, written));
	}
	edits
}

fn is_space(c: char) -> bool {
	c == ' ' || c == '\t'
}

/// The byte offset of character `index`
fn byte_at(characters: &[(char, bool)], index: usize) -> usize {
	characters[..index].iter().map(|(c, _)| c.len_utf8()).sum()
}

/// Runs of spaces become one; none just inside `(…)` and `[…]` or before a comma or semicolon; the spaces before a trailing comment stay
fn spaces(lines: &[String], scans: &[LineScan]) -> Vec<Edit> {
	rewrite_code(lines, scans, |characters, at, written, scan| {
		let c = characters[at].0;
		if !is_space(c) {
			written.push(c);
			return 1;
		}
		let run = characters[at..].iter().take_while(|(c, code)| is_space(*c) && *code).count();
		let next = characters.get(at + run).map(|(c, code)| (*c, *code));
		let before_comment = scan.comment == Some(byte_at(characters, at + run));
		let after_opener = written.ends_with(['(', '[']);
		match next {
			_ if before_comment => characters[at..at + run].iter().for_each(|(c, _)| written.push(*c)),
			None => {}
			Some((')' | ']' | ',' | ';', true)) => {}
			_ if after_opener => {}
			_ => written.push(' '),
		}
		run
	})
}

/// `a,b` → `a, b`, `a=1;b=2` → `a=1; b=2`
fn separator_spacing(lines: &[String], scans: &[LineScan]) -> Vec<Edit> {
	rewrite_code(lines, scans, |characters, at, written, _| {
		let c = characters[at].0;
		written.push(c);
		let next = characters.get(at + 1).map(|(c, _)| *c);
		if ",;".contains(c) && next.is_some_and(|next| !is_space(next) && !")]},;".contains(next)) {
			written.push(' ');
		}
		1
	})
}

/// `name:value` → `name: value`, a parameter's type too (`f(x: int)`, user 2026-10-15); `::`, `:=`, `://` and a colon
/// before a closer or the end stay
fn colon_spacing(lines: &[String], scans: &[LineScan]) -> Vec<Edit> {
	rewrite_code(lines, scans, |characters, at, written, _| {
		let c = characters[at].0;
		written.push(c);
		let previous = at.checked_sub(1).map(|before| characters[before].0);
		let next = characters.get(at + 1).map(|(c, _)| *c);
		let glued_key = previous.is_some_and(|before| !is_space(before) && before != ':');
		if c == ':' && glued_key && next.is_some_and(|next| !is_space(next) && !":=/)]}".contains(next)) {
			written.push(' ');
		}
		1
	})
}

fn is_operand_end(c: char) -> bool {
	c.is_alphanumeric() || OPERAND_ENDS.contains(c)
}

fn is_operand_start(c: char) -> bool {
	c.is_alphanumeric() || OPERAND_STARTS.contains(c)
}

/// The spaced operator starting at `at`, glued to no other operator character, between two operands
fn binary_operator(characters: &[(char, bool)], at: usize) -> Option<&'static str> {
	let longest = SPACED_OPERATORS.first().map_or(0, |operator| operator.chars().count());
	let rest: String = characters[at..].iter().take(longest).map(|(c, _)| c).collect();
	let operator = *SPACED_OPERATORS.iter().find(|operator| rest.starts_with(**operator))?;
	let after = at + operator.chars().count();
	let glued_to_operator = |index: Option<usize>| {
		index.and_then(|index| characters.get(index)).is_some_and(|(c, _)| OPERATOR_CHARACTERS.contains(*c))
	};
	if glued_to_operator(at.checked_sub(1)) || glued_to_operator(Some(after)) {
		return None;
	}
	let before = characters[..at].iter().rev().map(|(c, _)| *c).find(|c| !is_space(*c))?;
	let next = characters[after..].iter().map(|(c, _)| *c).find(|c| !is_space(*c))?;
	(is_operand_end(before) && is_operand_start(next)).then_some(operator)
}

fn operator_spacing(lines: &[String], scans: &[LineScan]) -> Vec<Edit> {
	rewrite_code(lines, scans, |characters, at, written, _| {
		let Some(operator) = binary_operator(characters, at) else {
			written.push(characters[at].0);
			return 1;
		};
		let kept = written.trim_end_matches([' ', '\t']).len();
		written.truncate(kept);
		written.push(' ');
		written.push_str(operator);
		written.push(' ');
		let length = operator.chars().count();
		length + characters[at + length..].iter().take_while(|(c, code)| is_space(*c) && *code).count()
	})
}

/// The indentation as written, a tab counting TAB_WIDTH
fn written_width(line: &str) -> usize {
	line.chars().take_while(|c| is_space(*c)).map(|c| if c == '\t' { TAB_WIDTH } else { 1 }).sum()
}

/// One tab per nesting level: a level for each line that opened brackets still open (`f({` opens one), less the
/// brackets the line closes first; a continued expression one more; an offside block (`def f(x):` and the lines
/// indented below it) one more for each block it is in
fn indentation(lines: &[String], scans: &[LineScan]) -> Vec<Edit> {
	let mut edits = Vec::new();
	let mut offside: Vec<usize> = Vec::new(); // the written widths of the open offside blocks' headers
	let mut previous_code = String::new();
	for (index, line) in lines.iter().enumerate() {
		let scan = &scans[index];
		let code = code_text(line, scan);
		if scan.starts_in_text_or_comment() || line.trim().is_empty() {
			if scan.starts_in_text_or_comment() && !code.is_empty() {
				previous_code = code;
			}
			continue;
		}
		let width = written_width(line);
		while offside.last().is_some_and(|&header| width <= header) {
			offside.pop();
		}
		let mut open = scan.open_before.clone();
		open.truncate(open.len().saturating_sub(leading_closers(line, scan)));
		open.dedup();
		let continued = !code.is_empty()
			&& (CONTINUING_ENDS.iter().any(|end| previous_code.ends_with(end))
				|| CONTINUING_STARTS.iter().any(|start| code.starts_with(start))
				|| code.starts_with("else") && previous_code.starts_with("then "));
		let level = open.len() + offside.len() + continued as usize;
		edits.extend(changed(index, line, format!("{}{}", INDENT.repeat(level), line.trim_start())));
		let closes_its_brackets =
			scans.get(index + 1).is_none_or(|next| next.open_before.len() == scan.open_before.len());
		if code.ends_with(':') && closes_its_brackets {
			offside.push(width);
		}
		if !code.is_empty() {
			previous_code = code;
		}
	}
	edits
}
