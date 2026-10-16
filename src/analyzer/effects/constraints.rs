//! Effect constraints the program declares (`f: Pure`, `main: IO`) and the violations the inferred effects give
use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
	pub allowed: EffectSet,
	/// Named effects allowed: `! ask` lets f emit ask
	pub events: BTreeSet<String>,
	pub line: usize,
	pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectViolation {
	pub function: String,
	pub constraint: Constraint,
	pub excess: EffectSet,
	pub excess_events: BTreeSet<String>,
	/// `f → helper → puts`: the calls through which the first excess effect enters
	pub chain: Vec<String>,
}

impl fmt::Display for EffectViolation {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"effect violation at {}:{}: {} is declared ! {} but performs {} via {}",
			self.constraint.line,
			self.constraint.column,
			self.function,
			effect_names(self.constraint.allowed, &self.constraint.events),
			effect_names(self.excess, &self.excess_events),
			self.chain
				.iter()
				.map(|name| crate::lowering::typed_library_words::written(name))
				.collect::<Vec<_>>()
				.join(" → ")
		)?;
		if self.excess.contains(Div) {
			write!(f, " (Div: may not terminate: a while loop, or recursion without a parameter that shrinks toward a guarded base case)")?;
		}
		Ok(())
	}
}

fn constraint_names(node: &Node) -> Option<Vec<String>> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(vec![name.clone()]),
		Node::List(items, _, _) => {
			items.iter().map(|item| constraint_names(item).and_then(|n| n.into_iter().next())).collect()
		}
		_ => None,
	}
}

fn is_effect_name(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if EffectSet::named(&[name]).is_some())
}

/// `definition ! Effects` → (definition, allowed effects)
fn split_constraint(node: &Node) -> Option<(&Node, (EffectSet, BTreeSet<String>))> {
	let Node::Key(definition, Op::Not, declared) = node.drop_meta() else {
		return None;
	};
	definition.defined_name()?;
	let names = constraint_names(declared)?;
	let allowed = allowed_names(&names)?;
	Some((definition, allowed))
}

/// The effects and events a constraint names: `Pure`, `IO`, and an event in lower case (`ask`); None for an unknown
/// capitalized name, which is no effect
fn allowed_names(names: &[String]) -> Option<(EffectSet, BTreeSet<String>)> {
	let mut events = BTreeSet::new();
	let mut allowed = EffectSet::PURE;
	for name in names {
		match EffectSet::named(&[name.as_str()]) {
			Some(effects) => allowed = allowed.union(effects),
			None if name.starts_with(|first: char| first.is_lowercase()) => {
				events.insert(name.clone());
			}
			None => return None,
		}
	}
	Some((allowed, events))
}

/// Each constrained statement with the bare effect names a comma split off after it:
/// `g(x) := … ! IO, FFI` parses as `[(g := … ! IO), FFI]`.
fn constrained_statements(items: &[Node]) -> Vec<(usize, usize)> {
	let mut statements = vec![];
	let mut index = 0;
	while index < items.len() {
		let trailing = if split_constraint(&items[index]).is_some() {
			items[index + 1..].iter().take_while(|item| is_effect_name(item)).count()
		} else {
			0
		};
		statements.push((index, trailing));
		index += 1 + trailing;
	}
	statements
}

pub(super) fn constraints(program: &Node) -> Vec<(String, Constraint)> {
	let mut found = vec![];
	collect_constraints(program, &mut found);
	found
}

fn collect_constraints(node: &Node, found: &mut Vec<(String, Constraint)>) {
	if let Some((definition, (allowed, events))) = split_constraint(node) {
		let (line, column) = node.get_lineinfo().map(|info| (info.line_nr, info.column)).unwrap_or_default();
		found.push((
			definition.defined_name().expect("checked by split_constraint"),
			Constraint { allowed, events, line, column },
		));
		return;
	}
	if let Node::List(items, _, _) = node.drop_meta() {
		for (index, trailing) in constrained_statements(items) {
			let before = found.len();
			collect_constraints(&items[index], found);
			let extra: Vec<&str> = items[index + 1..=index + trailing]
				.iter()
				.filter_map(|item| match item.drop_meta() {
					Node::Symbol(name) => Some(name.as_str()),
					_ => None,
				})
				.collect();
			if let (Some((_, constraint)), Some(more)) = (found.get_mut(before), EffectSet::named(&extra)) {
				constraint.allowed = constraint.allowed.union(more);
			}
		}
	}
}

/// The program with every `! Effects` constraint removed, ready for emission
pub fn without_constraints(node: Node) -> Node {
	if let Some((definition, _)) = split_constraint(&node) {
		return definition.clone();
	}
	match node {
		meta @ Node::Meta { .. } => meta.under_meta(without_constraints),
		Node::List(items, bracket, separator) => {
			let mut kept: Vec<Node> = constrained_statements(&items)
				.into_iter()
				.map(|(index, _)| without_constraints(items[index].clone()))
				.collect();
			if kept.len() == 1 && items.len() > 1 {
				return kept.remove(0); // only the comma split `f := … ! IO, FFI` made this a list
			}
			Node::List(kept, bracket, separator)
		}
		other => other,
	}
}
