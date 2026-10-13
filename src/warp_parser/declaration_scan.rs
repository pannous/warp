//! Scanning a source before parsing it: the operators it declares (`infix operator ⊕`, their precedence) and its type names, on its code without comments and texts
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum UserOperatorKind {
	Prefix,
	Suffix,
	Infix,
}

#[derive(Clone, Debug)]
pub(super) struct UserOperator {
	pub(super) glyph: Vec<char>,
	pub(super) kind: UserOperatorKind,
	/// How tight it binds: a suffix's power, a prefix's right power, an infix's left power (its right one is one more)
	pub(super) level: u8,
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

pub(super) const OPERATOR_FUNCTION_PREFIX: &str = "operator_";
pub(super) const OPERATOR_KINDS: [(&str, UserOperatorKind); 3] = [
	("prefix", UserOperatorKind::Prefix),
	("suffix", UserOperatorKind::Suffix),
	("infix", UserOperatorKind::Infix),
];
/// A user operator binds like the built-ins it resembles: suffix like the superscripts, prefix like unary minus, infix like `+`
pub(super) const USER_SUFFIX_BP: u8 = Op::Square.binding_power().0;
pub(super) const USER_PREFIX_RIGHT_BP: u8 = Op::Neg.binding_power().1;
pub(super) const USER_INFIX_BP: (u8, u8) = Op::Add.binding_power();
/// `operator ⊕ has precedence above *`: ⊕ binds this much tighter than `*`; the built-in levels are at least 5 apart
pub(super) const PRECEDENCE_STEP: u8 = 2;
pub(super) const PRECEDENCE_DIRECTIONS: [(&str, bool); 2] = [("above", true), ("below", false)];

/// A glyph a program may declare as an operator: symbols with a non-ASCII character (`‼`, `⊕`), never a letter or digit.
/// Superscript digits and signs too (P48, user-decided) (`suffix operator ³`, `prefix operator ⁻`, wiki/operator.md)
pub(super) fn is_operator_glyph(glyph: &str) -> bool {
	let is_superscript = |c: char| superscript_digit(c).is_some() || superscript_sign(c).is_some();
	!glyph.is_empty()
		&& glyph
			.chars()
			.all(|c| is_superscript(c) || (!c.is_alphanumeric() && !c.is_whitespace()))
		&& !glyph.is_ascii()
}

/// `suffix operator ⁰ := …` and the short `suffix ⁰ := …` (P48): the glyph and the words after `:=`
pub(super) fn declared_glyph<'a>(words: &'a [&'a str]) -> Option<&'a str> {
	match words {
		["operator", glyph, ":=", ..] | [glyph, ":=", ..] => Some(glyph),
		_ => None,
	}
}

/// The operator a `prefix|suffix|infix` declaration declares: a glyph (`infix operator ⊕ := …`), or for infix a word with
/// optional parameters, `infix operator divides(d:int, n:int) := …` (card g_mnvA); `infix divides(d, n) := …` is the
/// alias without `operator`
pub(super) fn declared_operator(kind: UserOperatorKind, words: &[&str]) -> Option<String> {
	if let Some(glyph) = declared_glyph(words).filter(|glyph| is_operator_glyph(glyph)) {
		return Some(glyph.to_string());
	}
	let head = match words {
		[OPERATOR_WORD, head, ..] | [head, ..] if kind == UserOperatorKind::Infix && words.contains(&":=") => head,
		_ => return None,
	};
	let word: String = head.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
	let parameters = &head[word.len()..];
	let is_word = word.starts_with(char::is_alphabetic) && word != OPERATOR_WORD;
	(is_word && (parameters.is_empty() || parameters.starts_with('('))).then_some(word)
}

/// `divides`: a declared operator that is a word, ending where the word ends (not `dividesx`)
pub(super) fn is_operator_word(glyph: &[char]) -> bool {
	glyph.first().is_some_and(|c| c.is_alphabetic())
}

pub(super) fn is_plain_name(word: &str) -> bool {
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
			let name: String = pair[1].chars().take_while(|ch| is_identifier_char(*ch)).collect();
			let rest = &pair[1][name.len()..];
			(rest.is_empty() || rest.starts_with(['(', '{', '<', '[', ':', ';'])).then_some(name)
		})
		.filter(|name| is_plain_name(name))
		.collect()
}

/// The source with its comments (`// …`, `/* … */`) and texts (`"…"`, `'…'`) blanked out
pub(super) fn code_only(source: &str) -> String {
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
		(from..chars.len())
			.find(|&i| chars[i..].starts_with(end))
			.map_or(chars.len(), |i| i + end.len())
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

/// The operators a source declares, and those of the standard modules it uses: `use regex` makes `matches` infix
/// (card matches-infix)
pub(super) fn scan_user_operators(source: &str) -> Vec<UserOperator> {
	let mut found = declared_operators(source);
	for module in used_std_modules(source) {
		found.extend(declared_operators(module));
	}
	found
}

/// The sources of the standard modules a source uses: `use regex`, `import regex`
fn used_std_modules(source: &str) -> Vec<&'static str> {
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

/// The operators a source itself declares: `prefix|suffix|infix operator ⊕ := body` and the pattern `a ⊕ b := body` (infix)
fn declared_operators(source: &str) -> Vec<UserOperator> {
	let mut found: Vec<UserOperator> = Vec::new();
	for statement in source.lines().flat_map(|line| line.split(';')) {
		let words: Vec<&str> = statement.split_whitespace().collect();
		let by_kind = words.split_first().and_then(|(word, rest)| {
			let (_, kind) = OPERATOR_KINDS.iter().find(|(keyword, _)| keyword == word)?;
			Some((declared_operator(*kind, rest)?, *kind))
		});
		let declared = by_kind.or_else(|| match words.as_slice() {
			[left, glyph, right, ":=", ..]
				if is_plain_name(left) && is_plain_name(right) && is_operator_glyph(glyph) =>
			{
				Some((glyph.to_string(), UserOperatorKind::Infix))
			}
			_ => None,
		});
		if let Some((glyph, kind)) = declared {
			found.push(UserOperator {
				glyph: glyph.chars().collect(),
				kind,
				level: kind.default_level(),
			});
		}
	}
	// `operator ⊕ has precedence above *`, wherever it stands, once every operator is known
	for statement in source.lines().flat_map(|line| line.split(';')) {
		let words: Vec<&str> = statement.split_whitespace().collect();
		if let Ok(Some((glyph, level))) = precedence_declaration(&words, &found) {
			found
				.iter_mut()
				.filter(|operator| operator.glyph == glyph.chars().collect::<Vec<char>>())
				.for_each(|operator| operator.level = level);
		}
	}
	found.sort_by_key(|operator| std::cmp::Reverse(operator.glyph.len()));
	found
}

/// `operator ⊕ has precedence above|below [operator] Y`: the glyph ⊕ and its new level; Ok(None) for any other statement,
/// Err for a precedence declaration that cannot be followed (a built-in ⊕, an unknown Y)
pub(super) fn precedence_declaration<'a>(
	words: &[&'a str],
	declared: &[UserOperator],
) -> Result<Option<(&'a str, u8)>, String> {
	let ["operator", glyph, "has", "precedence", direction, rest @ ..] = words else {
		return Ok(None);
	};
	let Some((_, above)) = PRECEDENCE_DIRECTIONS.iter().find(|(word, _)| word == direction) else {
		return Ok(None);
	};
	let other = match rest {
		["operator", other] | [other] => *other,
		_ => return Ok(None),
	};
	if !declared
		.iter()
		.any(|operator| operator.glyph.iter().collect::<String>() == *glyph)
	{
		return Err(format!("operator {glyph} is built in: only a declared operator gets a precedence (`infix operator {glyph} := …` first)"));
	}
	let reference = match declared
		.iter()
		.find(|operator| operator.glyph.iter().collect::<String>() == other)
	{
		Some(operator) => operator.level,
		None => built_in_level(other).ok_or_else(|| {
			format!("unknown operator {other} in `operator {glyph} has precedence {direction} {other}`")
		})?,
	};
	let level = if *above {
		reference.saturating_add(PRECEDENCE_STEP)
	} else {
		reference.saturating_sub(PRECEDENCE_STEP)
	};
	Ok(Some((glyph, level)))
}

/// The left binding power of a built-in infix operator written as `text` (`*`, `+`, `and`)
pub(super) fn built_in_level(text: &str) -> Option<u8> {
	let parser = WarpParser::new_with_options(
		text.to_string(),
		ParserOptions {
			data_mode: true,
			..ParserOptions::default()
		},
	);
	match parser.peek_operator() {
		Some((op, length)) if length == text.chars().count() => Some(op.binding_power().0),
		_ => None,
	}
}
