//! Operator alias lines (decision operator-alias): `operator × := *` spells an operator another way, keeping its
//! precedence; `prefix operator √ := sqrt` spells a prefix one; `circumfix operator ⌊ ⌋ := floor` brackets an
//! expression with a glyph pair that applies a function to it. lib/math.warp's lines are the math spellings every
//! program reads; a program's own lines are aliases of that program (WarpParser::source_aliases)
use super::Op;
use std::sync::OnceLock;

const MATH_SOURCE: &str = include_str!("../../lib/math.warp");
const OPERATOR_WORD: &str = "operator";
const PREFIX_WORD: &str = "prefix";
pub const CIRCUMFIX_WORD: &str = "circumfix";
const DEFINE: &str = ":=";
const STATEMENT_END: char = ';';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AliasPosition {
	Infix,
	Prefix,
}

#[derive(Clone, Debug)]
pub struct OperatorAlias {
	pub spelling: String,
	/// the spelling it stands for: `contains` for `∋`, which In reads by its word
	pub target: String,
	pub op: Op,
	pub position: AliasPosition,
}

/// `circumfix operator ⌊ ⌋ := floor`: ⌊x⌋ is floor(x)
#[derive(Clone, Debug)]
pub struct CircumfixAlias {
	pub opening: String,
	pub closing: String,
	pub target: String,
}

/// The spelling a lib/math.warp alias stands for: `in` for `∈`
pub fn alias_target(spelling: &str) -> Option<&'static str> {
	math_aliases().iter().find(|alias| alias.spelling == spelling).map(|alias| alias.target.as_str())
}

/// The alias lines of lib/math.warp, read once
pub fn math_aliases() -> &'static [OperatorAlias] {
	static ALIASES: OnceLock<Vec<OperatorAlias>> = OnceLock::new();
	ALIASES.get_or_init(|| source_aliases(MATH_SOURCE))
}

/// The circumfix lines of lib/math.warp, read once
pub fn math_circumfixes() -> &'static [CircumfixAlias] {
	static CIRCUMFIXES: OnceLock<Vec<CircumfixAlias>> = OnceLock::new();
	CIRCUMFIXES.get_or_init(|| source_circumfixes(MATH_SOURCE))
}

/// The alias lines of a source's code: an alias line in a text or a comment declares nothing
pub fn source_aliases(source: &str) -> Vec<OperatorAlias> {
	declared_in(source, alias_line)
}

/// The circumfix lines of a source's code
pub fn source_circumfixes(source: &str) -> Vec<CircumfixAlias> {
	declared_in(source, circumfix_line)
}

fn declared_in<T>(source: &str, declaration: fn(&str) -> Option<T>) -> Vec<T> {
	let code = crate::warp_parser::code_only(source);
	code.lines().flat_map(|line| line.split(STATEMENT_END)).filter_map(declaration).collect()
}

/// The statement declares an alias or a circumfix operator: nothing is left to define once the pre-scan read it
pub fn declares_alias(statement: &str) -> bool {
	alias_line(statement).is_some() || circumfix_line(statement).is_some()
}

/// `circumfix operator ⌊ ⌋ := floor`: the glyph pair a statement declares and the function it applies
pub fn circumfix_line(statement: &str) -> Option<CircumfixAlias> {
	let words: Vec<&str> = statement.split_whitespace().collect();
	let [CIRCUMFIX_WORD, OPERATOR_WORD, opening, closing, DEFINE, target] = words.as_slice() else {
		return None;
	};
	Some(CircumfixAlias { opening: opening.to_string(), closing: closing.to_string(), target: target.to_string() })
}

/// `operator × := *`, `prefix operator √ := sqrt`: the alias a statement declares; the target is a spelling of the
/// parser's own table (an infix operator) or a prefix operator word
pub fn alias_line(statement: &str) -> Option<OperatorAlias> {
	let words: Vec<&str> = statement.split_whitespace().collect();
	let (position, rest) = match words.as_slice() {
		[PREFIX_WORD, OPERATOR_WORD, rest @ ..] => (AliasPosition::Prefix, rest),
		[OPERATOR_WORD, rest @ ..] => (AliasPosition::Infix, rest),
		_ => return None,
	};
	let [spelling, DEFINE, target] = rest else {
		return None;
	};
	let op = match position {
		AliasPosition::Infix => super::table_operator(target)?,
		AliasPosition::Prefix => {
			crate::warp_parser::PREFIX_OPERATOR_WORDS.into_iter().find(|(word, _)| word == target)?.1
		}
	};
	Some(OperatorAlias { spelling: spelling.to_string(), target: target.to_string(), op, position })
}
