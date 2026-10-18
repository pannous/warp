//! Functions defined by pattern clauses (card pattern-clauses, decision pattern-roadmap): definitions of one name and
//! arity, one of them with a pattern parameter, are one function whose body tries the clauses in order, as a match does:
//! `fib(0) := 0; fib(1) := 1; fib(n) := fib(n-1) + fib(n-2)` is
//! `fib(n·1) := match n·1 { 0: 0; 1: 1; _: fib(n·1-1) + fib(n·1-2) }`, several parameters matched as one list
//! `match [x·1, y·2] { [0, _]: …; … }`. A plain parameter name matches anything: its arm holds `_` and its body reads
//! the joined parameter, so a global of the same name never turns it into a comparison. Typed parameters (`f(x: int)`)
//! stay overloads by type (overloads.rs).

use super::nodes::{is_call_head, key, matched};
use super::switch::WILDCARD;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

/// The name of a joined parameter no clause names plainly, numbered by its position
const ARGUMENT: &str = "argument";
const NUMBER_MARK: &str = "·";

pub fn lower(node: Node) -> Node {
	match node.map_children(lower) {
		Node::List(items, bracket, separator) => Node::List(joined_clauses(items), bracket, separator),
		other => other,
	}
}

/// The statements with each function's clauses joined at the place of its first one
fn joined_clauses(items: Vec<Node>) -> Vec<Node> {
	let mut clauses: HashMap<(String, usize), Vec<Clause>> = HashMap::new();
	for item in &items {
		if let Some(clause) = Clause::of(item) {
			clauses.entry((clause.name.clone(), clause.parameters.len())).or_default().push(clause);
		}
	}
	clauses.retain(|_, group| {
		group.len() > 1 && group.iter().any(Clause::has_pattern) && group.iter().all(Clause::is_joinable)
	});
	if clauses.is_empty() {
		return items;
	}
	let mut joined_signatures = HashSet::new();
	let mut result = Vec::with_capacity(items.len());
	for item in items {
		let Some(signature) = Clause::of(&item).map(|clause| (clause.name, clause.parameters.len())) else {
			result.push(item);
			continue;
		};
		match clauses.remove(&signature) {
			Some(group) => {
				result.push(joined(&signature.0, group));
				joined_signatures.insert(signature);
			}
			None if joined_signatures.contains(&signature) => {}
			None => result.push(item),
		}
	}
	result
}

struct Clause {
	name: String,
	parameters: Vec<Node>,
	body: Node,
}

impl Clause {
	/// `name(parameters…) := body`
	fn of(node: &Node) -> Option<Clause> {
		let Node::Key(head, Op::Define, body) = node.drop_meta() else {
			return None;
		};
		if !is_call_head(head) {
			return None;
		}
		let Node::List(items, _, _) = head.drop_meta() else {
			return None;
		};
		let (name, parameters) = items.split_first()?;
		let parameters = match parameters {
			// `f(n, [a, b])` may hold its parameters as one comma list
			[Node::List(group, Bracket::None, Separator::Comma)] => group.clone(),
			_ => parameters.to_vec(),
		};
		let name = name.symbol_name()?.to_string();
		(!parameters.is_empty()).then(|| Clause { name, parameters, body: body.as_ref().clone() })
	}

	fn has_pattern(&self) -> bool {
		self.parameters.iter().any(is_pattern)
	}

	/// No typed parameter: those overload by type
	fn is_joinable(&self) -> bool {
		!self.parameters.iter().any(
			|parameter| matches!(parameter.drop_meta(), Node::Key(name, Op::Colon, _) if plain_name(name).is_some()),
		)
	}

	/// The match arm: the patterns, `_` for a plain name, and the body reading the joined parameters; a clause of plain
	/// names only is the one `_` that takes any value
	fn arm(self, parameter_names: &[String]) -> Node {
		let takes_any = !self.has_pattern();
		let mut renamed = HashMap::new();
		let patterns: Vec<Node> = self
			.parameters
			.into_iter()
			.zip(parameter_names)
			.map(|(parameter, joined_name)| match plain_name(&parameter) {
				Some(name) => {
					renamed.insert(name.to_string(), Node::Symbol(joined_name.clone()));
					Node::Symbol(WILDCARD.into())
				}
				None => parameter,
			})
			.collect();
		let body = crate::law::substitute(&self.body, &renamed);
		let pattern = if takes_any { Node::Symbol(WILDCARD.into()) } else { one_or_list(patterns) };
		key(pattern, Op::Colon, body)
	}
}

/// A parameter that is not a plain name: a literal, a list, a map, code, a `$hole`
fn is_pattern(parameter: &Node) -> bool {
	plain_name(parameter).is_none() && !matches!(parameter.drop_meta(), Node::Key(_, Op::Colon, _))
}

fn plain_name(parameter: &Node) -> Option<&str> {
	parameter.symbol_name().filter(|name| !name.starts_with('$'))
}

/// One function of the clauses: numbered parameters and a match over them
fn joined(name: &str, clauses: Vec<Clause>) -> Node {
	let arity = clauses[0].parameters.len();
	let parameter_names: Vec<String> = (0..arity)
		.map(|position| {
			let plain = clauses.iter().find_map(|clause| plain_name(&clause.parameters[position]));
			format!("{}{NUMBER_MARK}{}", plain.unwrap_or(ARGUMENT), position + 1)
		})
		.collect();
	let subject = one_or_list(parameter_names.iter().map(|name| Node::Symbol(name.clone())).collect());
	// the clauses after one that takes any value are never tried
	let reached = clauses.iter().position(|clause| !clause.has_pattern()).map_or(clauses.len(), |last| last + 1);
	let arms = clauses.into_iter().take(reached).map(|clause| clause.arm(&parameter_names)).collect();
	let head = Node::List(
		std::iter::once(Node::Symbol(name.into())).chain(parameter_names.into_iter().map(Node::Symbol)).collect(),
		Bracket::Round,
		Separator::Space,
	);
	key(head, Op::Define, matched(subject, arms))
}

/// One parameter as itself, several as a list matched at once
fn one_or_list(mut nodes: Vec<Node>) -> Node {
	if nodes.len() == 1 {
		return nodes.remove(0);
	}
	Node::List(nodes, Bracket::Square, Separator::Comma)
}
