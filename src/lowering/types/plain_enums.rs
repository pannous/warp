//! Plain enums (card plain-enum, user decision enum-case B): `enum Coin { Penny, Nickel, Dime }` (Swift's `{ case north,
//! south }`, Kotlin's `enum class`) is the sum type of its cases, each its own value as a data-less variant of a tagged
//! union is (sum_variants.rs): `Coin.Dime`, `Coin::Dime` and Swift's `.dime` are `Dime`, and `Dime is Coin`. A case's
//! index, counting from 0 as C does (unlike warp's 1-based positions `x in xs`, `c#1`), is `Coin.Dime as int`; an int
//! becomes a case explicitly, `2 as Coin` or `Coin(2)`, an error when no case has that index.

use super::nodes::{key, matched};
use super::sum_variants::sum_type;
use crate::analyzer::match_exhaustive::typed_subjects;
use crate::errors::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::RAISE_WORD;
use crate::Number;
use std::collections::HashMap;

const ENUM_WORD: &str = "enum";
/// Swift's `enum Direction { case north, south }`
const CASE_WORD: &str = "case";
/// The types a case converts to and the number each counts its first case as: `Coin.Dime as int` and `as index` its
/// index 2 (from 0, as C counts), `as position` its position 3 (from 1, as `xs#p` counts; index_positions.rs)
const COUNT_TYPES: [(&str, i64); 3] = [("int", 0), ("index", 0), ("position", 1)];
const OTHER_ARM: &str = "_";

/// The plain enums of a program: each one's cases in order, by name
struct Enums {
	cases: HashMap<String, Vec<String>>,
	/// The names declared of an enum type, `c: Coin`
	subjects: HashMap<String, Option<String>>,
}

impl Enums {
	/// The enum having a case of that name, when only one has it
	fn owner(&self, case: &str) -> Option<&str> {
		let mut owners = self.cases.iter().filter(|(_, cases)| cases.iter().any(|named| named == case));
		match (owners.next(), owners.next()) {
			(Some((name, _)), None) => Some(name),
			_ => None,
		}
	}

	fn is_case_of(&self, enum_name: &Node, case: &Node) -> bool {
		self.cases.get(&enum_name.drop_meta().name()).is_some_and(|cases| cases.contains(&case.drop_meta().name()))
	}

	/// The enum of a value: a case by name, or a name declared of the enum
	fn type_of(&self, value: &Node) -> Option<&str> {
		let name = value.symbol_name()?;
		self.owner(name).or_else(|| self.subjects.get(name)?.as_deref())
	}
}

pub fn lower(program: Node) -> Node {
	let mut cases = HashMap::new();
	program.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			cases.extend(enum_cases(items));
		}
	});
	if cases.is_empty() {
		return program;
	}
	let subjects = typed_subjects(&program, |type_name| cases.contains_key(type_name));
	rewritten(program, &Enums { cases, subjects })
}

/// The name and cases of an enum declaration `enum Color {red, green}` (Swift's `{ case north, south }`)
fn enum_cases(items: &[Node]) -> Option<(String, Vec<String>)> {
	let [word, name, cases] = items else { return None };
	let name = name.symbol_name()?;
	let Node::List(cases, Bracket::Curly, _) = cases.drop_meta() else {
		return None;
	};
	let names: Option<Vec<String>> = cases.iter().map(case_name).collect();
	word.is_symbol(ENUM_WORD).then_some((name.to_string(), names?))
}

/// A case of an enum, `north` of Swift's `case north`
fn case_name(case: &Node) -> Option<String> {
	match case.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::List(words, _, _) => match words.as_slice() {
			[keyword, name] if keyword.is_symbol(CASE_WORD) => match name.drop_meta() {
				Node::Symbol(name) => Some(name.clone()),
				_ => None,
			},
			_ => None,
		},
		_ => None,
	}
}

fn rewritten(node: Node, enums: &Enums) -> Node {
	if let Node::List(items, _, _) = node.drop_meta() {
		if let Some((name, cases)) = enum_cases(items) {
			return sum_type(name, cases.into_iter().map(Node::Symbol).collect(), vec![]);
		}
	}
	match node.map_children(|child| rewritten(child, enums)) {
		Node::Key(owner, Op::Dot | Op::Scope, case) if enums.is_case_of(&owner, &case) => case.drop_meta().clone(),
		// Swift's `.north`, and its arm `case .north:` read as the member `case.north`
		Node::Key(owner, Op::Dot, case) if is_implicit_owner(&owner) && enums.owner(&case.name()).is_some() => {
			case.drop_meta().clone()
		}
		Node::Key(value, Op::As, target) => converted(*value, *target, enums),
		Node::List(items, Bracket::Round, separator) => match items.as_slice() {
			[enum_name, number] if enums.cases.contains_key(&enum_name.drop_meta().name()) => {
				case_at(number.clone(), &enum_name.drop_meta().name(), enums)
			}
			_ => Node::List(items, Bracket::Round, separator),
		},
		other => other,
	}
}

fn is_implicit_owner(owner: &Node) -> bool {
	matches!(owner.drop_meta(), Node::Empty) || owner.is_symbol(CASE_WORD)
}

/// `Coin.Dime as int` the case's index, `as position` its position, `2 as Coin` the case at an index; other
/// conversions as they are
fn converted(value: Node, target: Node, enums: &Enums) -> Node {
	let target_name = target.drop_meta().name();
	if enums.cases.contains_key(&target_name) {
		return case_at(value, &target_name, enums);
	}
	let count_type = COUNT_TYPES.iter().find(|(count_type, _)| *count_type == target_name);
	match enums.type_of(&value).zip(count_type) {
		Some((enum_name, (_, first))) => counted(value, &enums.cases[enum_name], *first),
		None => key(value, Op::As, target),
	}
}

/// The number of a case counting the first as `first`: `2` of `Dime` from 0, a match over the cases for a name of the
/// enum type
fn counted(value: Node, cases: &[String], first: i64) -> Node {
	let numbered = cases.iter().zip(first..);
	if let Some((_, number)) = numbered.clone().find(|(case, _)| value.drop_meta().name() == **case) {
		return Node::int(number);
	}
	let arms = numbered.map(|(case, number)| key(Node::Symbol(case.clone()), Op::Colon, Node::int(number))).collect();
	matched(value, arms)
}

/// The case at an index: `Dime` of `2`, a match over the indexes for any other value, an error past the cases
fn case_at(value: Node, enum_name: &str, enums: &Enums) -> Node {
	let cases = &enums.cases[enum_name];
	let no_case = format!(" is no {enum_name} case");
	if let Node::Number(Number::Int(number)) = value.drop_meta() {
		let found = usize::try_from(*number).ok().and_then(|index| cases.get(index));
		return match found {
			Some(case) => Node::Symbol(case.clone()),
			None => Diagnostic::at(&value, format!("{}{no_case}", value.serialize())).into_error(),
		};
	}
	let raised = Node::List(
		vec![Node::Symbol(RAISE_WORD.into()), key(value.clone(), Op::Add, Node::Text(no_case))],
		Bracket::None,
		Separator::Space,
	);
	let arms = cases
		.iter()
		.enumerate()
		.map(|(index, case)| key(Node::int(index as i64), Op::Colon, Node::Symbol(case.clone())))
		.chain([key(Node::Symbol(OTHER_ARM.into()), Op::Colon, raised)])
		.collect();
	matched(value, arms)
}
