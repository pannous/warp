//! Operator alias lines (decision operator-alias): `operator × := *` spells an operator another way, keeping its
//! precedence; `prefix operator √ := sqrt` spells a prefix one. lib/math.warp's lines are the math spellings every
//! program reads; a program's own lines are aliases of that program (WarpParser::source_aliases)
use super::Op;
use std::sync::OnceLock;

const MATH_SOURCE: &str = include_str!("../../lib/math.warp");
const OPERATOR_WORD: &str = "operator";
const PREFIX_WORD: &str = "prefix";
const DEFINE: &str = ":=";
const COMMENT: &str = " //";
const STATEMENT_END: char = ';';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AliasPosition {
	Infix,
	Prefix,
}

#[derive(Clone, Debug)]
pub struct OperatorAlias {
	pub spelling: String,
	pub op: Op,
	pub position: AliasPosition,
}

/// The alias lines of lib/math.warp, read once
pub fn math_aliases() -> &'static [OperatorAlias] {
	static ALIASES: OnceLock<Vec<OperatorAlias>> = OnceLock::new();
	ALIASES.get_or_init(|| source_aliases(MATH_SOURCE))
}

/// The alias lines of a source
pub fn source_aliases(source: &str) -> Vec<OperatorAlias> {
	source.lines().flat_map(|line| line.split(STATEMENT_END)).filter_map(alias_line).collect()
}

/// `operator × := *`, `prefix operator √ := sqrt`: the alias a statement declares; the target is a spelling of the
/// parser's own table (an infix operator) or a prefix operator word
pub fn alias_line(statement: &str) -> Option<OperatorAlias> {
	let code = statement.split(COMMENT).next()?;
	let words: Vec<&str> = code.split_whitespace().collect();
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
	Some(OperatorAlias { spelling: spelling.to_string(), op, position })
}
