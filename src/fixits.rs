//! Machine-applicable fixes (notes/fixits.md): a warning or an ambiguity error offers each reading the user might have
//! meant as a `Fix` (the text as written, its replacement), and a host turns it into an `Edit` of the source (byte
//! range + replacement): the playground's "I meant: …" buttons (src/web.rs), later `warp fix` and IDE quick fixes.
//! A replacement is an explicit form, which never warns, so applying a fix also ends its warning.

use std::ops::Range;

/// One reading the user might have meant: replace the text `written` (near the diagnostic's position) with `replacement`
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fix {
	pub meaning: String,
	pub written: String,
	pub replacement: String,
	/// Where `written` stands when that is not at the diagnostic (`n=0` of main for `global n=0`, asked in a function)
	pub at: Option<(usize, usize)>,
	/// Further edits the same reading needs elsewhere (a data key and its reads), each at its own place
	pub also: Vec<Fix>,
}

pub fn fix(meaning: impl Into<String>, written: impl Into<String>, replacement: impl Into<String>) -> Fix {
	Fix { meaning: meaning.into(), written: written.into(), replacement: replacement.into(), at: None, also: vec![] }
}

/// A fix that adds `line` above the first line of the source: a missing `use math`
pub fn added_first_line(meaning: impl Into<String>, line: impl Into<String>) -> Fix {
	Fix { meaning: meaning.into(), written: String::new(), replacement: format!("{}\n", line.into()), at: Some((1, 1)), also: vec![] }
}

impl Fix {
	/// The button text: "I meant: square(3) + square(4)", "add: use math"
	pub fn label(&self) -> String {
		match self.is_insertion() {
			true => format!("add: {}", self.replacement.trim_end()),
			false => format!("I meant: {}", self.replacement),
		}
	}

	/// Nothing written is replaced: the replacement goes in at its place (added_first_line)
	fn is_insertion(&self) -> bool {
		self.written.is_empty() && self.at.is_some()
	}

	pub fn at(self, line: usize, column: usize) -> Self {
		Fix { at: Some((line, column)), ..self }
	}

	/// The same reading also replaces `written` at `line`:`column` with `replacement`
	pub fn and(mut self, written: impl Into<String>, replacement: impl Into<String>, (line, column): (usize, usize)) -> Self {
		self.also.push(fix(self.meaning.clone(), written, replacement).at(line, column));
		self
	}

	/// A fix that changes nothing (the reading already written) is no fix; spacing counts: `1 -1` → `1 - 1`
	pub fn changes_something(&self) -> bool {
		self.is_insertion() || (!self.written.is_empty() && self.written.trim() != self.replacement.trim())
	}
}

/// A change of the source: replace the bytes `range` with `replacement`
#[derive(Clone, Debug, PartialEq)]
pub struct Edit {
	pub range: Range<usize>,
	pub replacement: String,
}

/// Spacing and list commas: the compiler shows `written` serialized (`[1 2] + 3`), the source may say `[1,2]+3`
fn is_insignificant(c: char) -> bool {
	c.is_whitespace() || c == ','
}

fn without_insignificant(text: &str) -> String {
	text.chars().filter(|c| !is_insignificant(*c)).collect()
}

fn is_word_char(c: char) -> bool {
	c.is_alphanumeric() || c == '_'
}

/// The byte end of `wanted` matched at `start`, ignoring insignificant characters on both sides; `None` when it does not match
fn match_at(source: &str, start: usize, wanted: &[char]) -> Option<usize> {
	let mut rest = wanted.iter().peekable();
	let mut end = start;
	for (offset, c) in source[start..].char_indices() {
		let Some(&&next) = rest.peek() else { break };
		if is_insignificant(c) {
			continue;
		}
		if c != next {
			return None;
		}
		rest.next();
		end = start + offset + c.len_utf8();
	}
	rest.peek().is_none().then_some(end)
}

/// Does the match stand as whole words: `n =` must not match inside `fn =`
fn on_word_boundaries(source: &str, range: &Range<usize>) -> bool {
	let first_inside = source[range.clone()].chars().next();
	let last_inside = source[range.clone()].chars().next_back();
	let before = source[..range.start].chars().next_back();
	let after = source[range.end..].chars().next();
	let glued = |inside: Option<char>, outside: Option<char>| inside.is_some_and(is_word_char) && outside.is_some_and(is_word_char);
	!glued(first_inside, before) && !glued(last_inside, after)
}

/// The 1-based line and column of a byte offset
pub fn line_and_column(source: &str, offset: usize) -> (usize, usize) {
	let before = &source[..offset];
	let line = before.matches('\n').count() + 1;
	let column = before.rsplit('\n').next().unwrap_or_default().chars().count() + 1;
	(line, column)
}

/// Where `written` stands in `source`: of all its places, the one nearest to the diagnostic's `line`:`column`
/// (0:0, no position: the first one)
pub fn locate(source: &str, line: usize, column: usize, written: &str) -> Option<Range<usize>> {
	let wanted: Vec<char> = without_insignificant(written).chars().collect();
	if wanted.is_empty() {
		return None;
	}
	let distance = |range: &Range<usize>| {
		let (at_line, at_column) = line_and_column(source, range.start);
		(at_line.abs_diff(line), at_column.abs_diff(column))
	};
	source.char_indices()
		.filter(|(_, c)| *c == wanted[0])
		.filter_map(|(start, _)| match_at(source, start, &wanted).map(|end| start..end))
		.filter(|range| on_word_boundaries(source, range))
		.min_by_key(|range| if line == 0 { (0, range.start) } else { distance(range) })
}

/// The byte offset of the 1-based `line`:`column`, `None` past the end of the source
fn offset_of(source: &str, line: usize, column: usize) -> Option<usize> {
	let line_start = match line {
		1 => 0,
		_ => source.match_indices('\n').nth(line - 2)?.0 + 1,
	};
	let line_text = source[line_start..].split('\n').next().unwrap_or_default();
	let within = line_text.char_indices().map(|(offset, _)| offset).chain([line_text.len()]).nth(column - 1)?;
	Some(line_start + within)
}

/// The edit that applies `fix` to `source` for a diagnostic at `line`:`column`, `None` when its text is not there
pub fn edit(source: &str, line: usize, column: usize, fix: &Fix) -> Option<Edit> {
	let (line, column) = fix.at.unwrap_or((line, column));
	if fix.is_insertion() {
		return offset_of(source, line, column).map(|offset| Edit { range: offset..offset, replacement: fix.replacement.clone() });
	}
	locate(source, line, column, &fix.written).map(|range| Edit { range, replacement: fix.replacement.clone() })
}

/// Every edit of `fix` (its own and those it also needs), the last in the source first so each applies to the source
/// as the earlier ones left it; `None` when any text is not there or two edits overlap
pub fn edits(source: &str, line: usize, column: usize, fix: &Fix) -> Option<Vec<Edit>> {
	let mut edits = std::iter::once(fix).chain(&fix.also).map(|part| edit(source, line, column, part)).collect::<Option<Vec<_>>>()?;
	edits.sort_by_key(|edit| std::cmp::Reverse(edit.range.start));
	let overlapping = edits.windows(2).any(|pair| pair[1].range.end > pair[0].range.start);
	(!overlapping).then_some(edits)
}

/// The source with `fix` of the diagnostic at `line`:`column` applied, `None` when its text is not there
/// (a later `warp fix` and IDE quick fixes start here)
pub fn fixed(source: &str, line: usize, column: usize, fix: &Fix) -> Option<String> {
	let edits = edits(source, line, column, fix)?;
	Some(edits.iter().fold(source.to_string(), |changed, edit| apply(&changed, edit)))
}

pub fn apply(source: &str, edit: &Edit) -> String {
	let mut changed = source.to_string();
	changed.replace_range(edit.range.clone(), &edit.replacement);
	changed
}

/// The offset in UTF-16 code units (JavaScript strings, LSP positions) of a byte offset
pub fn utf16_offset(source: &str, offset: usize) -> usize {
	source[..offset].encode_utf16().count()
}
