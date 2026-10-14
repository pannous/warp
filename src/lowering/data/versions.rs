//! Versions: `1.2.3` and `v1.2.3` literals, `version 1.2.3` values anywhere in code (`version` is a soft keyword: only before a
//! version, `version = 2` stays a variable), compared part by part as numbers (1.10 > 1.9, 1.0 = 1).
//! A module's top level `version 1.2.3` declares its version; `use x version 1.2.3`, `use x from 1.2.3` and
//! `use x >= 1.2.3` require one of its package.
use super::nodes::{children_rewritten, key};
use crate::node::Node;
use crate::operators::Op;
use crate::words::VERSION_KEYWORD;
use std::cmp::Ordering;
use std::fmt;

/// `use x from 1.2`: that version or a later one
pub const MINIMUM_KEYWORD: &str = "from";
/// tags `v1.2.3` name versions too
const TAG_PREFIX: char = 'v';

/// The digits as written (`1.10`), ordered by their numbers
#[derive(Clone, Debug)]
pub struct Version(String);

impl Version {
	pub fn parse(text: &str) -> Option<Version> {
		let digits = text.strip_prefix(TAG_PREFIX).unwrap_or(text);
		let numeric = digits.split('.').all(|part| part.parse::<u64>().is_ok());
		numeric.then(|| Version(digits.to_string()))
	}

	/// trailing zeros do not count: 1.0 is 1.0.0
	fn numbers(&self) -> Vec<u64> {
		let mut numbers: Vec<u64> = self.0.split('.').map(|part| part.parse().unwrap_or(0)).collect();
		while numbers.last() == Some(&0) {
			numbers.pop();
		}
		numbers
	}
}

impl Ord for Version {
	fn cmp(&self, other: &Self) -> Ordering {
		self.numbers().cmp(&other.numbers())
	}
}

impl PartialOrd for Version {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl PartialEq for Version {
	fn eq(&self, other: &Self) -> bool {
		self.cmp(other).is_eq()
	}
}

impl Eq for Version {}

impl fmt::Display for Version {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str(&self.0)
	}
}

/// The version of a package a program uses: exactly it (Op::Eq) or it or a later one (Op::Ge)
#[derive(Clone, Debug, PartialEq)]
pub struct Requirement(pub Op, pub Version);

impl Requirement {
	pub fn allows(&self, version: &Version) -> bool {
		self.0.holds(version.cmp(&self.1))
	}
}

impl fmt::Display for Requirement {
	fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		let minimum = if self.0 == Op::Ge { ">= " } else { "" };
		write!(formatter, "{VERSION_KEYWORD} {minimum}{}", self.1)
	}
}

/// Length of a version literal at the start: `1.2.3`, `10.0.1.4`, `v1.2.3` (at least two dots), else 0.
/// One dot stays a decimal number, `1..3` a range, `v2` a name.
pub fn literal_len(chars: &[char]) -> usize {
	dotted_len(chars, 2)
}

/// Length of the operand of the soft keyword `version`: digits and single dots (`version 1.10` keeps 1.10), else 0
pub fn operand_len(chars: &[char]) -> usize {
	dotted_len(chars, 0)
}

/// Digits separated by single dots, with at least `min_dots` of them (then a tag `v` may lead); 0 when a letter follows
/// (`1.2.3a`)
fn dotted_len(chars: &[char], min_dots: usize) -> usize {
	let mut end = usize::from(min_dots > 0 && chars.first() == Some(&TAG_PREFIX));
	let mut dots = 0;
	loop {
		let digits = chars[end..].iter().take_while(|c| c.is_ascii_digit()).count();
		if digits == 0 {
			return 0;
		}
		end += digits;
		if chars.get(end) != Some(&'.') || !chars.get(end + 1).is_some_and(char::is_ascii_digit) {
			break;
		}
		end += 1;
		dots += 1;
	}
	let word_follows = chars.get(end).is_some_and(|c| c.is_alphanumeric() || *c == '_');
	if word_follows || dots < min_dots {
		0
	} else {
		end
	}
}

/// The version a node spells: a version literal, `version x`, a number, a text
pub fn version_of(node: &Node) -> Option<Version> {
	match node.drop_meta() {
		Node::Symbol(text) if is_version_value(node) || text.starts_with(|c: char| c.is_ascii_digit()) => {
			Version::parse(text)
		}
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
	node.is_symbol(VERSION_KEYWORD)
}

/// The version a module declares with a top level `version 1.2.3`
pub fn declared_version(statements: &[Node]) -> Option<Version> {
	statements.iter().find(|statement| is_version_value(statement)).and_then(version_of)
}

/// `version 1.2.3` and a version literal (`v2` stays a name)
fn is_version_value(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(text) => literal_len(&text.chars().collect::<Vec<_>>()) == text.chars().count(),
		Node::List(items, _, _) => items.len() == 2 && is_version_keyword(&items[0]) && version_of(node).is_some(),
		_ => false,
	}
}

/// Versions become their text; a comparison of two versions becomes its truth value
pub fn lower_versions(node: Node) -> Node {
	match node {
		Node::Key(left, op, right) if op.is_comparison() && (is_version_value(&left) || is_version_value(&right)) => {
			match (version_of(&left), version_of(&right)) {
				(Some(left), Some(right)) => Node::from(op.holds(left.cmp(&right))),
				_ => key(lower_versions(*left), op, lower_versions(*right)),
			}
		}
		_ if is_version_value(&node) => Node::Text(version_of(&node).unwrap().to_string()),
		other => children_rewritten(other, lower_versions),
	}
}
