//! Scanning a source before parsing it: the operators it declares (`infix operator ⊕`, their precedence) and its type names, on its code without comments and texts
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum UserOperatorKind {
	Prefix,
	Suffix,
	Infix,
}

#[derive(Clone, Debug)]
pub(crate) struct UserOperator {
	pub(crate) glyph: Vec<char>,
	pub(crate) kind: UserOperatorKind,
	/// How tight it binds: a suffix's power, a prefix's right power, an infix's left power (its right one is one more)
	pub(crate) level: u8,
}

impl UserOperatorKind {
	pub(super) fn default_level(self) -> u8 {
		match self {
			UserOperatorKind::Suffix => USER_SUFFIX_BP,
			UserOperatorKind::Prefix => USER_PREFIX_RIGHT_BP,
			UserOperatorKind::Infix => USER_INFIX_BP.0,
		}
	}
}

pub(super) use crate::operators::OPERATOR_FUNCTION_PREFIX;
pub(super) const OPERATOR_KINDS: [(&str, UserOperatorKind); 3] =
	[("prefix", UserOperatorKind::Prefix), ("suffix", UserOperatorKind::Suffix), ("infix", UserOperatorKind::Infix)];
/// A user operator binds like the built-ins it resembles: suffix like the superscripts, prefix like unary minus, infix like `+`
pub(super) const USER_SUFFIX_BP: u8 = Op::Square.binding_power().0;
pub(super) const USER_PREFIX_RIGHT_BP: u8 = Op::Neg.binding_power().1;
pub(super) const USER_INFIX_BP: (u8, u8) = Op::Add.binding_power();
/// `operator ⊕ has precedence above *`: ⊕ binds this much tighter than `*`; the built-in levels are at least 5 apart
pub(super) const PRECEDENCE_STEP: u8 = 2;
pub(super) const PRECEDENCE_DIRECTIONS: [(&str, bool); 2] = [("above", true), ("below", false)];

/// `full-name`: plain names joined by hyphens
fn is_kebab_name(word: &str) -> bool {
	word.contains('-') && word.split('-').all(is_plain_name)
}

pub(crate) fn is_plain_name(word: &str) -> bool {
	!word.is_empty() && word.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// The type names a source declares: the word after `class`, `struct`, `record` or `type` (not the call `type(x)`),
/// outside comments and texts (`// the type end` declares nothing)
pub(super) fn scan_declared_types(source: &str) -> std::collections::HashSet<String> {
	let source = code_only(source);
	let words: Vec<&str> = source.split_whitespace().collect();
	words
		.windows(2)
		.filter(|pair| TYPE_DECLARATION_WORDS.contains(&pair[0]) || pair[0] == RECORD_WORD || pair[0] == "type")
		// the name right after the word: `class Point{`, `class Point(val x: Int)`, `record Point(int X)`, `class P:`;
		// `record = find(…)` declares nothing
		.filter_map(|pair| {
			let name: String = pair[1].chars().take_while(|ch| is_identifier_char(*ch) || *ch == '-').collect();
			let rest = &pair[1][name.len()..];
			(rest.is_empty() || rest.starts_with(['(', '{', '<', '[', ':', ';'])).then_some(name)
		})
		.filter(|name| is_plain_name(name) || is_kebab_name(name))
		.chain(scan_multi_word_types(&source).iter().map(|words| words.join("-")))
		.collect()
}

/// The functions a source defines, wherever: `f(x) := …`, `f x := …`, `def f(x) {…}`, a variable holding one
/// (`f = x => x`, `f = function …`). ` /name` after such a function is a path, also before its definition (card
/// name-after); inside a definition taking a parameter of that name it divides (WarpParser::parameters_in_scope)
pub(super) fn scan_function_names(source: &str) -> std::collections::HashSet<String> {
	code_only(source).split(['\n', ';']).filter_map(definition_in).collect()
}

fn leading_name(text: &str) -> String {
	text.trim_start().chars().take_while(|ch| is_identifier_char(*ch)).collect()
}

/// The names a parameter list binds: `a, b: int`, `a b`, `x`
pub(super) fn parameter_names(list: &str) -> Vec<String> {
	list.split([',', '(', ')', ' ', '\t']).map(leading_name).filter(|name| is_plain_name(name)).collect()
}

/// The name of the function `statement` defines: `def f(x)`, `f(x) := …`, `f = x => …`
fn definition_in(statement: &str) -> Option<String> {
	let statement = statement.trim();
	let first = statement.split_whitespace().next()?;
	let name = if crate::operators::is_function_keyword(first) {
		leading_name(&statement[first.len()..])
	} else if let Some((head, _)) = statement.split_once(":=") {
		leading_name(head)
	} else {
		let (head, value) = statement.split_once('=')?;
		starts_as_function(value.trim_start()).then(|| head.trim().to_string())?
	};
	is_plain_name(&name).then_some(name)
}

/// A value that starts as a function: `x => …`, `(a, b) -> …`, `function (a) {…}`
fn starts_as_function(value: &str) -> bool {
	let Some(first) = value.split_whitespace().next() else { return false };
	if crate::operators::is_function_keyword(first) {
		return true;
	}
	let head_end =
		if value.starts_with('(') { value.find(')').map_or(0, |end| end + 1) } else { leading_name(value).len() };
	let arrow = value[head_end..].trim_start();
	head_end > 0 && (arrow.starts_with("=>") || arrow.starts_with("->"))
}

/// Words that stand between a type's name and its body, never part of the name: `class Dog extends Animal {`
const WORDS_BEFORE_TYPE_BODY: [&str; 6] =
	[EXTENDS_KEYWORD, WITH_KEYWORD, IMPLEMENTS_WORD, GO_INTERFACE_WORD, GC_WORD, GO_STRUCT_WORD];

/// The type names of several words a source declares, by their words: `class full name {…}` declares `full name`, written
/// `full-name` (card multi-word-class); only on one line, up to the body's `{`
pub(super) fn scan_multi_word_types(source: &str) -> Vec<Vec<String>> {
	let source = code_only(source);
	let mut types: Vec<Vec<String>> = source.lines().flat_map(multi_word_types_in_line).collect();
	// `full name` before `full`: the longest first
	types.sort_by_key(|words| std::cmp::Reverse(words.len()));
	types
}

fn multi_word_types_in_line(line: &str) -> Vec<Vec<String>> {
	let words: Vec<&str> = line.split_whitespace().collect();
	let is_name_word = |word: &str| is_plain_name(word) && !WORDS_BEFORE_TYPE_BODY.contains(&word);
	let mut types = vec![];
	for (at, keyword) in words.iter().enumerate() {
		if !TYPE_DECLARATION_WORDS.contains(keyword) && *keyword != RECORD_WORD && *keyword != "type" {
			continue;
		}
		let mut name = vec![];
		for word in &words[at + 1..] {
			let (word_part, glued_body) = match word.strip_suffix('{') {
				Some(glued) => (glued, true),
				None => (*word, false),
			};
			if *word == "{" || glued_body {
				if !word_part.is_empty() && is_name_word(word_part) {
					name.push(word_part.to_string());
				}
				if name.len() > 1 {
					types.push(std::mem::take(&mut name));
				}
				break;
			}
			if !is_name_word(word) {
				break;
			}
			name.push(word.to_string());
		}
	}
	types
}

/// The source with its comments (`// …`, `/* … */`) and texts (`"…"`, `'…'`) blanked out
pub(crate) fn code_only(source: &str) -> String {
	let chars: Vec<char> = source.chars().collect();
	let mut code = String::with_capacity(source.len());
	let mut at = 0;
	for (start, end) in comment_and_text_spans(source) {
		code.extend(&chars[at..start]);
		code.push(' ');
		at = end;
	}
	code.extend(&chars[at..]);
	code
}

/// Where the comments and texts of a source start and end, in characters; the lexer's first pass (card self-lexer,
/// lib/compiler/lexer.warp spans is its warp twin)
pub fn comment_and_text_spans(source: &str) -> Vec<(usize, usize)> {
	let chars: Vec<char> = source.chars().collect();
	let mut spans = Vec::new();
	let mut at = 0;
	while at < chars.len() {
		match text_or_comment_end(&chars, at) {
			Some(end) => {
				spans.push((at, end));
				at = end;
			}
			None => at += 1,
		}
	}
	spans
}

/// The end of the comment (`// …`, `/* … */`) or text (`"…"`, `'…'`) starting at `chars[at]`, if one starts there
pub(crate) fn text_or_comment_end(chars: &[char], at: usize) -> Option<usize> {
	let (c, next) = (chars[at], chars.get(at + 1).copied());
	let skip_until = |from: usize, end: &[char]| {
		(from..chars.len()).find(|&i| chars[i..].starts_with(end)).map_or(chars.len(), |i| i + end.len())
	};
	let after = match (c, next) {
		('/', Some('/')) if at == 0 || chars[at - 1] != ':' => {
			(at..chars.len()).find(|&i| chars[i] == '\n').unwrap_or(chars.len())
		}
		('/', Some('*')) => skip_until(at + 2, &['*', '/']),
		('"' | '\'', _) => quoted_end(chars, at + 1, c),
		_ => return None,
	};
	Some(after.max(at + 1))
}

/// The end of a quoted text from `from` on, past its closing quote: an escaped quote `\"` leaves it open
fn quoted_end(chars: &[char], from: usize, quote: char) -> usize {
	let mut at = from;
	while at < chars.len() {
		match chars[at] {
			'\\' => at += 2,
			c if c == quote => return at + 1,
			_ => at += 1,
		}
	}
	chars.len()
}

/// The sources of the standard modules a source uses: `use regex`, `import regex`
pub(crate) fn used_std_modules(source: &str) -> Vec<&'static str> {
	source
		.lines()
		.flat_map(|line| line.split(';'))
		.filter_map(|statement| match statement.split_whitespace().collect::<Vec<_>>().as_slice() {
			[keyword, module] if crate::modules::USE_KEYWORDS.contains(keyword) => {
				crate::modules::std_module_source(module)
			}
			_ => None,
		})
		.collect()
}
