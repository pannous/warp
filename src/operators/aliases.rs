//! Operator alias lines (decision operator-alias): `operator × := *` spells an operator another way, keeping its
//! precedence; `prefix operator √ := sqrt` spells a prefix one. lib/math.warp's lines are the math spellings every
//! program reads; a program's own lines are aliases of that program (WarpParser::source_aliases)
use super::Op;
use std::sync::OnceLock;

const MATH_SOURCE: &str = include_str!("../../lib/math.warp");
const OPERATOR_WORD: &str = "operator";
const PREFIX_WORD: &str = "prefix";
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

/// The spelling a lib/math.warp alias stands for: `in` for `∈`
pub fn alias_target(spelling: &str) -> Option<&'static str> {
	math_aliases().iter().find(|alias| alias.spelling == spelling).map(|alias| alias.target.as_str())
}

/// The alias lines of lib/math.warp, read once
pub fn math_aliases() -> &'static [OperatorAlias] {
	static ALIASES: OnceLock<Vec<OperatorAlias>> = OnceLock::new();
	ALIASES.get_or_init(|| source_aliases(MATH_SOURCE))
}

/// The alias lines of a source's code: an alias line in a text or a comment declares nothing
pub fn source_aliases(source: &str) -> Vec<OperatorAlias> {
	let code = crate::warp_parser::code_only(source);
	code.lines().flat_map(|line| line.split(STATEMENT_END)).filter_map(alias_line).collect()
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
