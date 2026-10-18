//! Exhaustive matches (card match-exhaustive, user decision match-exhaustive): a match on a name declared of a sum type
//! (`s: Shape` of `enum Shape { Circle(r), Dot }`, or of a plain enum `c: Coin`, plain_enums.rs) without a catch-all that misses a
//! case is the compile error "match on Shape misses Dot". A name only ever assigned cases of one type (`c = Dime`) is
//! of that type too, and its match must hold the cases assigned to it (card exhaustive-assigned): `c = green` may skip
//! blue, never green. A subject of unknown type, or an arm this check cannot read
//! (a guard, alternatives, a comparison), keeps the run-time error "no case for …".

use crate::errors::diagnostic::Diagnostic;
use crate::lowering::sum_variants::sum_types;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::words::{IS_WORD, SWITCH_WORDS};
use std::collections::HashMap;

/// Kotlin's `when (c) { … }`, matched like `match`
const KOTLIN_MATCH_WORD: &str = "when";

pub fn check(program: Node) -> Node {
	let cases: HashMap<String, Vec<String>> =
		sum_types(&program).into_iter().map(|(name, variants)| (name, variants.names())).collect();
	if cases.is_empty() {
		return program;
	}
	let mut subjects = typed_subjects(&program, |type_name| cases.contains_key(type_name));
	let mut held = HashMap::new();
	for (name, assigned) in assigned_subjects(&program, &cases) {
		if subjects.contains_key(&name) {
			continue;
		}
		subjects.insert(name.clone(), assigned.as_ref().map(|(type_name, _)| type_name.clone()));
		if let Some((_, assigned_cases)) = assigned {
			held.insert(name, assigned_cases);
		}
	}
	let mut problem = None;
	program.visit(&mut |node| {
		if problem.is_none() {
			problem = incomplete(node, &cases, &subjects, &held);
		}
	});
	problem.unwrap_or(program)
}

/// The names declared of a type `is_type` accepts, `s: Shape`, with their type; a name declared of two types has none
pub(crate) fn typed_subjects(program: &Node, is_type: impl Fn(&str) -> bool) -> HashMap<String, Option<String>> {
	let mut subjects: HashMap<String, Option<String>> = HashMap::new();
	program.visit(&mut |node| {
		if let Node::Key(name, Op::Colon, type_name) = node {
			if let (Node::Symbol(name), Node::Symbol(type_name)) = (name.drop_meta(), type_name.drop_meta()) {
				if is_type(type_name) {
					let declared = subjects.entry(name.clone()).or_insert(Some(type_name.clone()));
					if declared.as_ref() != Some(type_name) {
						*declared = None;
					}
				}
			}
		}
	});
	subjects
}

/// The names assigned cases of one type, `c = Dime`, with that type and the cases assigned; a name also assigned
/// anything else has none
fn assigned_subjects(
	program: &Node,
	cases: &HashMap<String, Vec<String>>,
) -> HashMap<String, Option<(String, Vec<String>)>> {
	let type_of_case = |case: &Node| {
		let case = case.drop_meta().symbol_name()?;
		let mut types = cases.iter().filter(|(_, names)| names.iter().any(|name| name == case));
		match (types.next(), types.next()) {
			(Some((type_name, _)), None) => Some(type_name.clone()),
			_ => None,
		}
	};
	let mut subjects: HashMap<String, Option<(String, Vec<String>)>> = HashMap::new();
	program.visit(&mut |node| {
		if let Node::Key(name, Op::Assign, value) = node {
			if let Some(name) = name.drop_meta().symbol_name() {
				let assigned = type_of_case(value);
				let known =
					subjects.entry(name.to_string()).or_insert(assigned.clone().map(|type_name| (type_name, vec![])));
				match (known.as_mut(), assigned) {
					(Some((type_name, held)), Some(assigned)) if *type_name == assigned => {
						held.push(value.drop_meta().name())
					}
					_ => *known = None,
				}
			}
		}
	});
	subjects
}

/// The error of a match on a typed subject whose arms name some of its cases and no catch-all
fn incomplete(
	node: &Node,
	cases: &HashMap<String, Vec<String>>,
	subjects: &HashMap<String, Option<String>>,
	held: &HashMap<String, Vec<String>>,
) -> Option<Node> {
	let (word, subject, arms) = subject_and_arms(node)?;
	let type_name = subjects.get(&subject)?.as_ref()?;
	let type_cases = &cases[type_name];
	let named: Vec<String> = arms
		.iter()
		.map(|arm| match arm.drop_meta() {
			Node::Key(pattern, _, _) => case_named(pattern, type_name),
			_ => None,
		})
		.collect::<Option<_>>()?;
	if !named.iter().all(|name| type_cases.contains(name)) {
		return None;
	}
	let missing: Vec<&str> = type_cases
		.iter()
		.filter(|case| !named.contains(case))
		.filter(|case| held.get(&subject).is_none_or(|held| held.contains(case)))
		.map(String::as_str)
		.collect();
	if missing.is_empty() {
		return None;
	}
	let arms_to_add: Vec<String> = missing.iter().map(|case| format!("{case}: …")).collect();
	let message = format!("match on {type_name} misses {}", missing.join(", "));
	let fix = format!("add {} or other: …", arms_to_add.join("; "));
	Some(Diagnostic::at(word, message).fix(fix).into_error())
}

/// `match s { … }`, `match (s { … })`, `when (s) { … }`: the word (where the match is written), the subject's name and
/// the arms
fn subject_and_arms(node: &Node) -> Option<(&Node, String, &[Node])> {
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	let word = items.first()?;
	let written = word.drop_meta().name();
	if !SWITCH_WORDS.contains(&written.as_str()) && written != KOTLIN_MATCH_WORD {
		return None;
	}
	let (subject, arms) = match &items[1..] {
		[subject, arms] => (subject, arms),
		[grouped] => match grouped.drop_meta() {
			Node::List(pair, _, _) if pair.len() == 2 => (&pair[0], &pair[1]),
			_ => return None,
		},
		_ => return None,
	};
	let Node::List(arms, Bracket::Curly, _) = arms.drop_meta() else {
		return None;
	};
	Some((word, subject_name(subject)?, arms))
}

/// `s`, or Kotlin's `(s)`
fn subject_name(subject: &Node) -> Option<String> {
	match subject.ungrouped() {
		Node::Symbol(name) => Some(name.clone()),
		_ => None,
	}
}

/// The case an arm's pattern names: `Dot`, `Circle(r)`, `Shape::Dot`, `Coin.Penny`, `is Circle`; none for a catch-all
/// or a pattern of another form
fn case_named(pattern: &Node, type_name: &str) -> Option<String> {
	match pattern.drop_meta() {
		Node::Symbol(name) if crate::lowering::switch::DEFAULT_KEYS.contains(&name.as_str()) => None,
		Node::Symbol(name) => Some(name.clone()),
		Node::List(items, Bracket::Round, _) => match items.as_slice() {
			[head, ..] if matches!(head.drop_meta(), Node::Symbol(_)) => Some(head.drop_meta().name()),
			_ => None,
		},
		Node::List(items, _, _) => match items.as_slice() {
			[word, case] if word.is_symbol(IS_WORD) => case_named(case, type_name),
			_ => None,
		},
		Node::Key(owner, Op::Scope | Op::Dot, case) if owner.drop_meta().name() == type_name => {
			case_named(case, type_name)
		}
		_ => None,
	}
}
