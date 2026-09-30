//! Versions: `1.2.3` and `v1.2.3` literals, `version 1.2.3` values anywhere in code (`version` is a soft keyword: only before a
//! version, `version = 2` stays a variable), compared part by part as numbers (1.10 > 1.9).
//! A module's top level `version 1.2.3` declares its version; `use x version 1.2.3`, `use x from 1.2.3` and
//! `use x >= 1.2.3` require one of its package.
use crate::node::Node;
use crate::operators::Op;
use std::cmp::Ordering;
use std::fmt;

pub const VERSION_KEYWORD: &str = "version";
/// `use x from 1.2`: that version or a later one
pub const MINIMUM_KEYWORD: &str = "from";
/// tags `v1.2.3` name versions too
const TAG_PREFIX: char = 'v';

#[derive(Clone, Debug)]
pub struct Version {
	parts: Vec<u64>,
	text: String,
}

impl Version {
	pub fn parse(text: &str) -> Option<Version> {
		let digits = text.strip_prefix(TAG_PREFIX).unwrap_or(text);
		let parts: Option<Vec<u64>> = digits.split('.').map(|part| part.parse().ok()).collect();
		Some(Version { parts: parts?, text: digits.to_string() })
	}

	/// trailing zeros do not count: 1.0 is 1.0.0
	fn significant_parts(&self) -> &[u64] {
		let length = self.parts.iter().rposition(|part| *part != 0).map_or(0, |last| last + 1);
		&self.parts[..length]
	}
}

impl PartialEq for Version {
	fn eq(&self, other: &Self) -> bool {
		self.significant_parts() == other.significant_parts()
	}
}

impl Eq for Version {}

impl Ord for Version {
	fn cmp(&self, other: &Self) -> Ordering {
		self.significant_parts().cmp(other.significant_parts())
	}
}

impl PartialOrd for Version {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl fmt::Display for Version {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str(&self.text)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum Requirement {
	Exact(Version),
	Minimum(Version),
}

impl Requirement {
	pub fn allows(&self, version: &Version) -> bool {
		match self {
			Requirement::Exact(wanted) => version == wanted,
			Requirement::Minimum(minimum) => version >= minimum,
		}
	}
}

impl fmt::Display for Requirement {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Requirement::Exact(version) => write!(formatter, "{VERSION_KEYWORD} {version}"),
			Requirement::Minimum(version) => write!(formatter, "{VERSION_KEYWORD} >= {version}"),
		}
	}
}

/// Length of a version literal at the start: digits with at least two dots (`1.2.3`, `10.0.1.4`), else 0.
/// One dot stays a decimal number, `1..3` a range.
pub fn literal_len(chars: &[char]) -> usize {
	match dotted_digits(chars) {
		(length, dots) if dots >= 2 => length,
		_ => 0,
	}
}

/// Length of a git tag style version literal at the start: `v1.2.3`, the `v` and a version literal
pub fn tagged_literal_len(chars: &[char]) -> usize {
	if chars.first() != Some(&TAG_PREFIX) {
		return 0;
	}
	match literal_len(&chars[1..]) {
		0 => 0,
		length => length + 1,
	}
}

/// Length of the operand of the soft keyword `version`: digits and single dots (`version 1.10` keeps 1.10), else 0
pub fn operand_len(chars: &[char]) -> usize {
	dotted_digits(chars).0
}

/// Digits separated by single dots, and the number of dots; nothing when a letter follows (`1.2.3a`)
fn dotted_digits(chars: &[char]) -> (usize, usize) {
	let mut length = 0;
	let mut dots = 0;
	loop {
		let digits = chars[length..].iter().take_while(|c| c.is_ascii_digit()).count();
		if digits == 0 {
			return (0, 0);
		}
		length += digits;
		let continues = chars.get(length) == Some(&'.') && chars.get(length + 1).is_some_and(char::is_ascii_digit);
		if !continues {
			let word_follows = chars.get(length).is_some_and(|c| c.is_alphanumeric() || *c == '_');
			return if word_follows { (0, 0) } else { (length, dots) };
		}
		length += 1;
		dots += 1;
	}
}

/// The version a node spells: a version literal, `version x`, a number, a text
pub fn version_of(node: &Node) -> Option<Version> {
	match node.drop_meta() {
		Node::Symbol(text) if is_version_literal(text) || text.starts_with(|c: char| c.is_ascii_digit()) => Version::parse(text),
		Node::Text(text) => Version::parse(text),
		Node::Number(_) => Version::parse(node.drop_meta().serialize().trim()),
		Node::List(items, _, _) => match items.as_slice() {
			[keyword, version] if is_version_keyword(keyword) => version_of(version),
			_ => None,
		},
		_ => None,
	}
}

pub fn is_version_keyword(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(keyword) if keyword == VERSION_KEYWORD)
}

/// `1.2.3` and `v1.2.3` as the lexer reads them; `v2` stays a name
fn is_version_literal(text: &str) -> bool {
	let chars: Vec<char> = text.chars().collect();
	chars.len() == literal_len(&chars).max(tagged_literal_len(&chars))
}

/// The version a module declares with a top level `version 1.2.3`
pub fn declared_version(statements: &[Node]) -> Option<Version> {
	statements.iter().find(|statement| is_version_value(statement)).and_then(version_of)
}

/// `version 1.2.3` and a version literal
fn is_version_value(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(text) => is_version_literal(text),
		Node::List(items, _, _) => items.len() == 2 && is_version_keyword(&items[0]) && version_of(node).is_some(),
		_ => false,
	}
}

/// Versions become their text; a comparison of two versions becomes its truth value
pub fn lower_versions(node: Node) -> Node {
	match node {
		Node::Key(left, op, right) if op.is_comparison() && (is_version_value(&left) || is_version_value(&right)) => {
			match (version_of(&left), version_of(&right)) {
				(Some(left), Some(right)) => compare(&left, op, &right),
				_ => Node::Key(Box::new(lower_versions(*left)), op, Box::new(lower_versions(*right))),
			}
		}
		_ if is_version_value(&node) => Node::Text(version_of(&node).unwrap().to_string()),
		Node::Key(left, op, right) => Node::Key(Box::new(lower_versions(*left)), op, Box::new(lower_versions(*right))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lower_versions).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower_versions(*node)), data },
		other => other,
	}
}

fn compare(left: &Version, op: Op, right: &Version) -> Node {
	let truth = match op {
		Op::Lt => left < right,
		Op::Gt => left > right,
		Op::Le => left <= right,
		Op::Ge => left >= right,
		Op::Ne => left != right,
		_ => left == right,
	};
	if truth { Node::True } else { Node::False }
}
