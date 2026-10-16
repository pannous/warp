//! What each line of warp source holds for the layout: its code (outside texts and comments), its trailing comment,
//! and the brackets open before it

/// The brackets that open and close the nesting the indentation follows
const OPENERS: [char; 3] = ['(', '[', '{'];
const CLOSERS: [char; 3] = [')', ']', '}'];

#[derive(Clone, Debug, Default)]
pub(super) struct LineScan {
	/// what the line starts inside: a text or a block comment owns its start, not the layout
	pub starts_inside: Inside,
	/// what the line ends inside (trailing spaces in a text or a block comment may be theirs)
	pub ends_inside: Inside,
	/// byte ranges of code: outside texts and comments
	pub code: Vec<(usize, usize)>,
	/// where a comment to the end of the line starts
	pub comment: Option<usize>,
	/// for each bracket open at the line's start, the line that opened it
	pub open_before: Vec<usize>,
}

impl LineScan {
	pub fn is_code(&self, at: usize) -> bool {
		self.code.iter().any(|&(start, end)| at >= start && at < end)
	}

	/// the line starts inside a text or a block comment
	pub fn starts_in_text_or_comment(&self) -> bool {
		self.starts_inside != Inside::Code
	}

	/// the line ends inside a text or a block comment
	pub fn ends_in_text_or_comment(&self) -> bool {
		self.ends_inside != Inside::Code
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) enum Inside {
	#[default]
	Code,
	Text(char),
	BlockComment(&'static str),
}

/// `#use lib`, `#include x`: a directive at the line start is a comment too (warp_parser/comments.rs)
const DIRECTIVES: [&str; 3] = ["use", "include", "import"];

/// `//` or `#` starting a comment at `at`: `//` anywhere but in a URL's `://` (decision floor-division); `#` after a
/// space or at the line start, before a space, `!`, `#` or the end (`# note`, `#!`, `##`), or a directive. `xs#1` and
/// `#xs` (a count) are code
fn starts_line_comment(line: &str, at: usize) -> bool {
	let before_is_space = at == 0 || line[..at].ends_with([' ', '\t']);
	let after = &line[at..];
	let next = |prefix: &str| after[prefix.len()..].chars().next();
	if after.starts_with("//") {
		return !line[..at].ends_with(':');
	}
	if after.starts_with('#') && before_is_space {
		let directive = line[..at].trim().is_empty() && DIRECTIVES.iter().any(|word| after[1..].starts_with(word));
		return directive || next("#").is_none_or(|c| matches!(c, ' ' | '!' | '#'));
	}
	false
}

/// The scan of every line, carrying texts, block comments and open brackets from line to line
pub(super) fn scan(lines: &[String]) -> Vec<LineScan> {
	let mut inside = Inside::Code;
	let mut open: Vec<usize> = Vec::new();
	let mut scans = Vec::with_capacity(lines.len());
	for (index, line) in lines.iter().enumerate() {
		let mut scan = LineScan { starts_inside: inside, open_before: open.clone(), ..Default::default() };
		let mut code_start = matches!(inside, Inside::Code).then_some(0);
		let mut chars = line.char_indices().peekable();
		while let Some((at, c)) = chars.next() {
			match inside {
				Inside::Text(quote) => {
					if c == '\\' {
						chars.next();
					} else if c == quote {
						inside = Inside::Code;
						code_start = Some(at + c.len_utf8());
					}
				}
				Inside::BlockComment(closing) => {
					if line[at..].starts_with(closing) {
						chars.nth(closing.len() - 2);
						inside = Inside::Code;
						code_start = Some(at + closing.len());
					}
				}
				Inside::Code => {
					let rest = &line[at..];
					let opens_block = ["/*", "/#"].into_iter().find(|opening| rest.starts_with(opening));
					if c == '"' || c == '\'' || c == '`' || opens_block.is_some() {
						if let Some(start) = code_start.take() {
							scan.code.push((start, at));
						}
						inside = match opens_block {
							Some("/*") => Inside::BlockComment("*/"),
							Some(_) => Inside::BlockComment("#/"),
							None => Inside::Text(c),
						};
						if opens_block.is_some() {
							chars.next();
						}
					} else if starts_line_comment(line, at) {
						if let Some(start) = code_start.take() {
							scan.code.push((start, at));
						}
						scan.comment = Some(at);
						break;
					} else if OPENERS.contains(&c) {
						open.push(index);
					} else if CLOSERS.contains(&c) {
						open.pop();
					}
				}
			}
		}
		if let (Some(start), None) = (code_start, scan.comment) {
			scan.code.push((start, line.len()));
		}
		scan.ends_inside = inside;
		scans.push(scan);
	}
	scans
}

/// The brackets the code of the line closes before anything else: `})` closes two
pub(super) fn leading_closers(line: &str, scan: &LineScan) -> usize {
	let Some(&(start, end)) = scan.code.first() else { return 0 };
	if !line[..start].trim().is_empty() {
		return 0;
	}
	line[start..end]
		.trim_start()
		.chars()
		.take_while(|c| CLOSERS.contains(c) || *c == ' ')
		.filter(|c| CLOSERS.contains(c))
		.count()
}

/// The code of the line without its trailing comment, trimmed (texts blanked to `""`, also one the line starts or
/// ends in: `y = x + "` ends in an operand, not in `+`; a block comment it starts or ends in is dropped)
pub(super) fn code_text(line: &str, scan: &LineScan) -> String {
	let blank_text = "\"\"";
	let is_text = |inside: Inside| matches!(inside, Inside::Text(_));
	let mut text = String::new();
	for (index, &(start, end)) in scan.code.iter().enumerate() {
		if index > 0 || is_text(scan.starts_inside) {
			text.push_str(blank_text);
		}
		text.push_str(&line[start..end]);
	}
	if is_text(scan.ends_inside) {
		text.push_str(blank_text);
	}
	text.trim().to_string()
}

/// For each character of `code`, whether it is warp code: outside texts and comments
pub(crate) fn code_characters(code: &str) -> Vec<bool> {
	let lines: Vec<String> = code.split('\n').map(str::to_string).collect();
	lines
		.iter()
		.zip(scan(&lines))
		.flat_map(|(line, scan)| line.char_indices().map(move |(at, _)| scan.is_code(at)).chain([false]))
		.collect()
}
