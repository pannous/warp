//! Effect inference (DESIGN.md "Effects", "Effects as enforced capabilities").
//!
//! A function's effects are the union of the trusted signatures of everything it calls,
//! resolved to a fixpoint so recursion terminates. The resolved external calls also decide
//! which WASM imports a module declares: no IO call, no WASI import.
//! Later the `EffectSet` moves onto semantic `Expr`/`FunctionDecl`; the report stays the query API.

use crate::analyzer::{extract_ffi_imports, extract_user_functions};
use crate::context::Context;
use crate::node::Node;
use crate::operators::{is_function_keyword, Op};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use Capability::*;
use Effect::*;

const PURE: &str = "Pure";
const ENTRY: &str = "main";
const QUERY_WORDS: [&str; 2] = ["effects", "of"];
const DECLARATION_KEYWORDS: [&str; 2] = ["import", "use"];
const STATE_KEYWORD: &str = "global";

/// Trusted effect signatures of the built-in host and WASI functions.
const TRUSTED_EXTERNALS: &[(&str, Capability, &[Effect])] = &[
	("fetch", Host, &[IO]),
	("puts", Wasi, &[IO]),
	("puti", Wasi, &[IO]),
	("putl", Wasi, &[IO]),
	("putf", Wasi, &[IO]),
	("fd_write", Wasi, &[IO, Unsafe]),
];

/// Closed effect set; `Pure` is the empty `EffectSet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effect {
	State,
	Allocation,
	IO,
	FFI,
	Async,
	Unsafe,
}

impl Effect {
	pub const ALL: [Effect; 6] = [State, Allocation, IO, FFI, Async, Unsafe];

	fn bit(self) -> u8 {
		1 << self as u8
	}

	pub fn named(name: &str) -> Option<Effect> {
		Self::ALL.into_iter().find(|effect| format!("{effect:?}") == name)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct EffectSet(u8);

impl EffectSet {
	pub const PURE: EffectSet = EffectSet(0);

	pub fn of(effects: &[Effect]) -> Self {
		effects.iter().copied().collect()
	}

	pub fn union(self, other: EffectSet) -> Self {
		EffectSet(self.0 | other.0)
	}

	pub fn minus(self, other: EffectSet) -> Self {
		EffectSet(self.0 & !other.0)
	}

	pub fn contains(self, effect: Effect) -> bool {
		self.0 & effect.bit() != 0
	}

	pub fn is_pure(self) -> bool {
		self == Self::PURE
	}

	pub fn is_subset_of(self, other: EffectSet) -> bool {
		self.minus(other).is_pure()
	}

	pub fn iter(self) -> impl Iterator<Item = Effect> {
		Effect::ALL.into_iter().filter(move |effect| self.contains(*effect))
	}

	/// `Pure` or effect names; `None` if any name is not an effect
	pub fn named(names: &[&str]) -> Option<EffectSet> {
		names.iter().map(|name| if *name == PURE { Some(Self::PURE) } else { Effect::named(name).map(|e| Self::of(&[e])) })
			.try_fold(Self::PURE, |all, one| one.map(|one| all.union(one)))
	}

	/// Structured query answer: `Pure`, `IO` or `(IO FFI)`
	pub fn to_node(self) -> Node {
		let mut names: Vec<Node> = self.iter().map(|effect| Node::Symbol(format!("{effect:?}"))).collect();
		match names.len() {
			0 => Node::Symbol(PURE.into()),
			1 => names.remove(0),
			_ => Node::List(names, crate::node::Bracket::Round, crate::node::Separator::Space),
		}
	}
}

impl FromIterator<Effect> for EffectSet {
	fn from_iter<I: IntoIterator<Item = Effect>>(effects: I) -> Self {
		EffectSet(effects.into_iter().fold(0, |bits, effect| bits | effect.bit()))
	}
}

impl fmt::Display for EffectSet {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if self.is_pure() {
			return write!(f, "{PURE}");
		}
		let names: Vec<String> = self.iter().map(|effect| format!("{effect:?}")).collect();
		write!(f, "{}", names.join(", "))
	}
}

/// Which import module an external function comes from
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
	Host,
	Wasi,
	Ffi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct External {
	pub capability: Capability,
	pub effects: EffectSet,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FunctionEffects {
	/// Inferred: union over everything this function calls, transitively
	pub effects: EffectSet,
	/// Directly resolved callees (user functions and externals)
	pub calls: BTreeSet<String>,
	pub declared: Option<Constraint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
	pub allowed: EffectSet,
	pub line: usize,
	pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectViolation {
	pub function: String,
	pub constraint: Constraint,
	pub excess: EffectSet,
	/// `f → helper → puts`: the calls through which the first excess effect enters
	pub chain: Vec<String>,
}

impl fmt::Display for EffectViolation {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "effect violation at {}:{}: {} is declared ! {} but performs {} via {}",
			self.constraint.line, self.constraint.column, self.function,
			self.constraint.allowed, self.excess, self.chain.join(" → "))
	}
}

#[derive(Debug, Clone, Default)]
pub struct EffectReport {
	/// User functions plus the pseudo-function `main` for top-level code
	pub functions: BTreeMap<String, FunctionEffects>,
	/// Every external function resolved anywhere in the module
	pub externals: BTreeMap<String, External>,
	pub violations: Vec<EffectViolation>,
	/// `! Effects` attached to something that is not a function definition
	pub misplaced: Vec<(String, Constraint)>,
}

impl EffectReport {
	pub fn of(program: &Node) -> Self {
		let mut context = Context::new();
		extract_user_functions(&mut context, program);
		extract_ffi_imports(&mut context, program);
		let resolver = Resolver { context: &context };

		let mut report = EffectReport::default();
		for (name, definition) in &context.user_functions {
			report.functions.insert(name.clone(), resolver.function(&definition.body));
		}
		report.functions.insert(ENTRY.into(), resolver.function(program));
		for (name, constraint) in constraints(program) {
			match report.functions.get_mut(&name) {
				Some(function) if name != ENTRY => function.declared = Some(constraint),
				_ => report.misplaced.push((name, constraint)),
			}
		}
		report.externals = report.functions.values().flat_map(|f| f.calls.iter())
			.filter_map(|name| resolver.external(name).map(|external| (name.clone(), external)))
			.collect();
		report.infer_to_fixpoint();
		report.violations = report.check_constraints();
		report
	}

	fn infer_to_fixpoint(&mut self) {
		loop {
			let mut changed = false;
			let names: Vec<String> = self.functions.keys().cloned().collect();
			for name in names {
				let inferred = self.functions[&name].calls.iter()
					.fold(self.functions[&name].effects, |all, callee| all.union(self.effects_of(callee).unwrap_or_default()));
				let function = self.functions.get_mut(&name).expect("known function");
				changed |= function.effects != inferred;
				function.effects = inferred;
			}
			if !changed {
				return;
			}
		}
	}

	fn check_constraints(&self) -> Vec<EffectViolation> {
		self.functions.iter().filter_map(|(name, function)| {
			let constraint = function.declared.clone()?;
			let excess = function.effects.minus(constraint.allowed);
			let first_excess = excess.iter().next()?;
			Some(EffectViolation { function: name.clone(), constraint, excess, chain: self.call_chain(name, first_excess) })
		}).collect()
	}

	/// Resolved effects of a user function, external, or `main`
	pub fn effects_of(&self, name: &str) -> Option<EffectSet> {
		self.functions.get(name).map(|f| f.effects)
			.or_else(|| self.externals.get(name).map(|e| e.effects))
			.or_else(|| trusted_external(name).map(|e| e.effects))
	}

	pub fn entry_effects(&self) -> EffectSet {
		self.functions[ENTRY].effects
	}

	/// Whether the module must import from this capability's module
	pub fn needs(&self, capability: Capability) -> bool {
		self.externals.values().any(|external| external.capability == capability)
	}

	pub fn calls_external(&self, name: &str) -> bool {
		self.externals.contains_key(name)
	}

	/// Shortest call path from `from` to the external that introduces `effect`
	pub fn call_chain(&self, from: &str, effect: Effect) -> Vec<String> {
		let mut predecessor: BTreeMap<String, String> = BTreeMap::new();
		let mut queue = VecDeque::from([from.to_string()]);
		while let Some(current) = queue.pop_front() {
			if self.externals.get(&current).is_some_and(|external| external.effects.contains(effect)) {
				let mut chain = vec![current.clone()];
				while let Some(previous) = predecessor.get(chain.last().expect("non-empty")) {
					chain.push(previous.clone());
				}
				chain.reverse();
				return chain;
			}
			for callee in self.functions.get(&current).map(|f| &f.calls).into_iter().flatten() {
				if callee != from && !predecessor.contains_key(callee) {
					predecessor.insert(callee.clone(), current.clone());
					queue.push_back(callee.clone());
				}
			}
		}
		vec![from.to_string()]
	}

	/// Diagnostic for the first violated constraint, else the answer to a trailing `effects of f`
	pub fn answer(&self, program: &Node) -> Option<Node> {
		if let Some(violation) = self.violations.first() {
			return Some(Node::Error(Box::new(Node::Text(violation.to_string()))));
		}
		if let Some((name, constraint)) = self.misplaced.first() {
			return Some(Node::Error(Box::new(Node::Text(format!(
				"misplaced effect constraint at {}:{}: {name} is not a function, `! {}` needs a function definition",
				constraint.line, constraint.column, constraint.allowed)))));
		}
		let name = effects_query(last_statement(program))?;
		Some(match self.effects_of(&name) {
			Some(effects) => effects.to_node(),
			None => Node::Error(Box::new(Node::Text(format!("effects of {name}: unknown function")))),
		})
	}
}

/// Rust API: resolved effects of `function` in `code`
pub fn effects_of(code: &str, function: &str) -> Option<EffectSet> {
	EffectReport::of(&crate::wasp_parser::WaspParser::parse(code)).effects_of(function)
}

fn trusted_external(name: &str) -> Option<External> {
	TRUSTED_EXTERNALS.iter().find(|(known, _, _)| *known == name)
		.map(|(_, capability, effects)| External { capability: *capability, effects: EffectSet::of(effects) })
}

struct Resolver<'a> {
	context: &'a Context,
}

impl Resolver<'_> {
	fn external(&self, name: &str) -> Option<External> {
		if self.context.user_functions.contains_key(name) {
			return None;
		}
		if self.context.ffi_imports.contains_key(name) {
			return Some(External { capability: Ffi, effects: EffectSet::of(&[FFI]) });
		}
		trusted_external(name)
	}

	fn resolves(&self, name: &str) -> bool {
		self.context.user_functions.contains_key(name) || self.external(name).is_some()
	}

	fn function(&self, body: &Node) -> FunctionEffects {
		let mut function = FunctionEffects::default();
		self.collect(body, &mut function);
		function
	}

	/// Every symbol naming a known function counts as a call; nested definitions and
	/// import declarations are skipped (definitions are analyzed on their own).
	fn collect(&self, node: &Node, function: &mut FunctionEffects) {
		if self.is_definition(node) || is_declaration(node) {
			return;
		}
		match node.drop_meta() {
			Node::Symbol(name) if name == STATE_KEYWORD => function.effects = function.effects.union(EffectSet::of(&[State])),
			Node::Symbol(name) if self.resolves(name) => {
				function.calls.insert(name.clone());
			}
			Node::List(items, _, _) => items.iter().for_each(|item| self.collect(item, function)),
			Node::Key(left, _, right) => {
				self.collect(left, function);
				self.collect(right, function);
			}
			Node::Error(inner) => self.collect(inner, function),
			_ => {}
		}
	}

	fn is_definition(&self, node: &Node) -> bool {
		defined_name(node).is_some_and(|name| self.context.user_functions.contains_key(&name))
	}
}

/// Name bound by `f(x) := …`, `f := …it…` or `def f(x){…}`
fn defined_name(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Key(left, Op::Define | Op::Assign, _) => leading_symbol(left),
		Node::List(items, _, _) if items.len() >= 2 && matches!(items[0].drop_meta(), Node::Symbol(s) if is_function_keyword(s)) => leading_symbol(&items[1]),
		_ => None,
	}
}

fn leading_symbol(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::List(items, _, _) => leading_symbol(items.first()?),
		_ => None,
	}
}

/// `import f from lib` / `use lib`
fn is_declaration(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(items, _, _) => matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if DECLARATION_KEYWORDS.contains(&word.as_str())),
		_ => false,
	}
}

fn effect_names(node: &Node) -> Option<Vec<String>> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(vec![name.clone()]),
		Node::List(items, _, _) => items.iter().map(|item| effect_names(item).and_then(|n| n.into_iter().next())).collect(),
		_ => None,
	}
}

fn is_effect_name(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if EffectSet::named(&[name]).is_some())
}

/// `definition ! Effects` → (definition, allowed effects)
fn split_constraint(node: &Node) -> Option<(&Node, EffectSet)> {
	let Node::Key(definition, Op::Not, declared) = node.drop_meta() else { return None };
	defined_name(definition)?;
	let names = effect_names(declared)?;
	let allowed = EffectSet::named(&names.iter().map(String::as_str).collect::<Vec<_>>())?;
	Some((definition, allowed))
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

fn constraints(program: &Node) -> Vec<(String, Constraint)> {
	let mut found = vec![];
	collect_constraints(program, &mut found);
	found
}

fn collect_constraints(node: &Node, found: &mut Vec<(String, Constraint)>) {
	if let Some((definition, allowed)) = split_constraint(node) {
		let (line, column) = node.get_lineinfo().map(|info| (info.line_nr, info.column)).unwrap_or_default();
		found.push((defined_name(definition).expect("checked by split_constraint"), Constraint { allowed, line, column }));
		return;
	}
	if let Node::List(items, _, _) = node.drop_meta() {
		for (index, trailing) in constrained_statements(items) {
			let before = found.len();
			collect_constraints(&items[index], found);
			let extra: Vec<&str> = items[index + 1..=index + trailing].iter().filter_map(|item| match item.drop_meta() {
				Node::Symbol(name) => Some(name.as_str()),
				_ => None,
			}).collect();
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
		Node::Meta { node, data } => Node::Meta { node: Box::new(without_constraints(*node)), data },
		Node::List(items, bracket, separator) => {
			let mut kept: Vec<Node> = constrained_statements(&items).into_iter().map(|(index, _)| without_constraints(items[index].clone())).collect();
			if kept.len() == 1 && items.len() > 1 {
				return kept.remove(0); // only the comma split `f := … ! IO, FFI` made this a list
			}
			Node::List(kept, bracket, separator)
		}
		other => other,
	}
}

fn last_statement(program: &Node) -> &Node {
	match program.drop_meta() {
		Node::List(items, _, _) if matches!(items.last().map(Node::drop_meta), Some(Node::List(..))) => items.last().expect("non-empty"),
		_ => program,
	}
}

/// `effects of f` → `f`
fn effects_query(statement: &Node) -> Option<String> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let words: Vec<&str> = items.iter().filter_map(|item| match item.drop_meta() {
		Node::Symbol(word) => Some(word.as_str()),
		_ => None,
	}).collect();
	match words.as_slice() {
		[effects, of, name] if items.len() == 3 && [*effects, *of] == QUERY_WORDS => Some(name.to_string()),
		_ => None,
	}
}
