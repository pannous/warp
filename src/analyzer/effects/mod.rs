//! Effect inference (DESIGN.md "Effects", "Effects as enforced capabilities").
//!
//! A function's effects are the union of the trusted signatures of everything it calls,
//! resolved to a fixpoint so recursion terminates. The resolved external calls also decide
//! which WASM imports a module declares: no IO call, no WASI import.
//! `Div` (as in Koka) marks code that may not terminate: a `while` loop, recursion that does not
//! demonstrably shrink a parameter toward a guarded base case, or mutual recursion. When unsure, Div.
//! Later the `EffectSet` moves onto semantic `Expr`/`FunctionDecl`; the report stays the query API.

use crate::analyzer::{extract_ffi_imports, function_definitions};
use crate::compiler::context::Context;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use Capability::*;
use Effect::*;

mod constraints;
pub use constraints::*;
mod events;
pub use events::*;
mod queries;
pub use queries::*;
mod resolver;
use resolver::*;
mod termination;
pub(crate) use termination::*;

const PURE: &str = "Pure";
const ENTRY: &str = "main";
pub(crate) const EFFECTS_WORD: &str = "effects";
const STATE_KEYWORD: &str = "global";

/// Closed effect set; `Pure` is the empty `EffectSet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effect {
	State,
	Allocation,
	IO,
	FFI,
	Async,
	Unsafe,
	/// May diverge: not provably terminating (Koka's `div`)
	Div,
	/// Runs a block known only at run time (`interpret e`, host run_block): its code is not known to the compiler
	Eval,
	/// May raise an error: `raise`, `error "…"`, a division by a variable (card fail-effect); `try` handles it
	Fail,
}

impl Effect {
	pub const ALL: [Effect; 9] = [State, Allocation, IO, FFI, Async, Unsafe, Div, Eval, Fail];

	const fn bit(self) -> u16 {
		1 << self as u8
	}

	pub fn named(name: &str) -> Option<Effect> {
		Self::ALL.into_iter().find(|effect| format!("{effect:?}") == name)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct EffectSet(u16);

const FAILS: EffectSet = EffectSet(Fail.bit());

impl EffectSet {
	pub const PURE: EffectSet = EffectSet(0);
	/// Construction only uses memory: reported (`effects of f` says Allocation) but pure (wiki/pure.md), so it is never
	/// a violation of `! Pure` and never keeps a function from being folded or memoized
	const UNOBSERVABLE: EffectSet = EffectSet(Allocation.bit());

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

	/// Pure as far as running it twice goes: a failure is deterministic, so a failing function folds and memoizes like
	/// a pure one (the same Error each time); `! Pure` refuses it all the same (check_constraints)
	pub fn is_pure(self) -> bool {
		self.observable().minus(FAILS) == Self::PURE
	}

	/// The effects but those only the memory use shows
	pub fn observable(self) -> Self {
		self.minus(Self::UNOBSERVABLE)
	}

	pub fn is_subset_of(self, other: EffectSet) -> bool {
		self.minus(other).is_pure()
	}

	pub fn iter(self) -> impl Iterator<Item = Effect> {
		Effect::ALL.into_iter().filter(move |effect| self.contains(*effect))
	}

	/// `Pure` or effect names; `None` if any name is not an effect
	pub fn named(names: &[&str]) -> Option<EffectSet> {
		names
			.iter()
			.map(|name| if *name == PURE { Some(Self::PURE) } else { Effect::named(name).map(|e| Self::of(&[e])) })
			.try_fold(Self::PURE, |all, one| one.map(|one| all.union(one)))
	}
}

impl FromIterator<Effect> for EffectSet {
	fn from_iter<I: IntoIterator<Item = Effect>>(effects: I) -> Self {
		EffectSet(effects.into_iter().fold(0, |bits, effect| bits | effect.bit()))
	}
}

impl fmt::Display for EffectSet {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if *self == Self::PURE {
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
	/// The pure math functions of libm (P26), linked like Ffi but granted to every program, untrusted ones included
	Libm,
	/// Runs typed `sql` templates (`execute`)
	Sql,
	/// Runs typed `sh` commands (`exec`)
	Process,
	/// Modules of another runtime, `use python "math"` (foreign_call, src/modules/foreign.rs; user decision P89)
	Foreign,
}

impl Capability {
	pub const ALL: [Capability; 7] = [Host, Wasi, Ffi, Libm, Sql, Process, Foreign];
	/// Capabilities `eval` grants: all of them for now (user decision P88, "currently allow everything to everyone");
	/// the checks stay, a host that grants less will narrow this
	pub const GRANTED_BY_EVAL: [Capability; 7] = Capability::ALL;
	/// Capabilities untrusted code gets: all of them for now too (P88)
	pub const GRANTED_UNTRUSTED: [Capability; 7] = Capability::ALL;
	/// Capabilities of a program run with `warp --sandbox`, as warp-lambda hosts strangers' programs on pannous.com
	/// (user, 2026-10-09: sandboxed): no C, no shell, no other runtime; its files and SQLite stay in its own folder by
	/// the systemd unit around it (web/hosting/server)
	pub const GRANTED_SANDBOXED: [Capability; 4] = [Host, Wasi, Libm, Sql];

	pub fn name(self) -> &'static str {
		match self {
			Host => "host",
			Wasi => "wasi",
			Ffi => "ffi",
			Libm => "libm",
			Sql => "sql",
			Process => "process",
			Foreign => "foreign",
		}
	}
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
	/// Performed by the body itself, not via a call: `global` state, a `while` loop, non-shrinking recursion
	pub own: EffectSet,
	/// Directly resolved callees (user functions and externals)
	pub calls: BTreeSet<String>,
	/// The callees called only inside `try`, which handles their Fail
	pub tried_calls: BTreeSet<String>,
	pub declared: Option<Constraint>,
	/// The events the function emits (card effect-handlers): named effects
	pub events: BTreeSet<String>,
	/// The events its body itself emits
	pub own_events: BTreeSet<String>,
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
		let mut context = function_definitions(program);
		extract_ffi_imports(&mut context, program);
		crate::analyzer::extract_host_words(&mut context, program);
		let resolver = Resolver { context: &context };

		let mut report = EffectReport::default();
		let user_globals = main_level_globals(program);
		for (name, definition) in &context.user_functions {
			let mut function = resolver.function(&definition.body);
			let parameters: Vec<&str> = definition.params.iter().map(|param| param.name.as_str()).collect();
			if reads_global(&definition.body, &user_globals, &parameters) {
				function.perform(State); // shared state read at call time (wiki/charged.md §3): never folded nor memoized
			}
			if function.calls.contains(name) && !recursion_shrinks(name, &parameters, &definition.body) {
				function.perform(Div);
			}
			report.functions.insert(name.clone(), function);
		}
		report.functions.insert(ENTRY.into(), resolver.function(program));
		report.mark_mutual_recursion();
		for (name, constraint) in constraints(program) {
			match report.functions.get_mut(&name) {
				Some(function) if name != ENTRY => function.declared = Some(constraint),
				_ => report.misplaced.push((name, constraint)),
			}
		}
		report.externals = report
			.functions
			.values()
			.flat_map(|f| f.calls.iter())
			.filter_map(|name| resolver.external(name).map(|external| (name.clone(), external)))
			.collect();
		report.infer_to_fixpoint();
		report.with_events_of(program)
	}

	/// A cycle through another function has no measure we can check: Div
	fn mark_mutual_recursion(&mut self) {
		let cyclic: Vec<String> = self
			.functions
			.iter()
			.filter(|(name, function)| {
				function.calls.iter().any(|callee| callee != *name && self.reaches(callee, name))
			})
			.map(|(name, _)| name.clone())
			.collect();
		for name in cyclic {
			self.functions.get_mut(&name).expect("known function").perform(Div);
		}
	}

	/// Whether `to` is reachable from `from` over resolved user-function calls
	fn reaches(&self, from: &str, to: &str) -> bool {
		let mut seen = BTreeSet::new();
		let mut queue = VecDeque::from([from.to_string()]);
		while let Some(current) = queue.pop_front() {
			if current == to {
				return true;
			}
			if seen.insert(current.clone()) {
				queue.extend(self.functions.get(&current).into_iter().flat_map(|f| f.calls.iter().cloned()));
			}
		}
		false
	}

	fn infer_to_fixpoint(&mut self) {
		loop {
			let mut changed = false;
			let names: Vec<String> = self.functions.keys().cloned().collect();
			for name in names {
				let function = &self.functions[&name];
				let inferred = function.calls.iter().fold(function.effects, |all, callee| {
					let effects = self.effects_of(callee).unwrap_or_default();
					all.union(if function.tried_calls.contains(callee) { effects.minus(FAILS) } else { effects })
				});
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
		self.functions
			.iter()
			.filter_map(|(name, function)| {
				let constraint = function.declared.clone()?;
				let excess = function.effects.minus(constraint.allowed).observable();
				let excess_events: BTreeSet<String> = function.events.difference(&constraint.events).cloned().collect();
				let chain = match (excess.iter().next(), excess_events.iter().next()) {
					(Some(effect), _) => self.call_chain(name, effect),
					(None, Some(event)) => {
						self.chain_to(name, |function| function.own_events.contains(event), |_| false)
					}
					(None, None) => return None,
				};
				Some(EffectViolation { function: name.clone(), constraint, excess, excess_events, chain })
			})
			.collect()
	}

	/// Resolved effects of a user function, external, or `main`
	pub fn effects_of(&self, name: &str) -> Option<EffectSet> {
		self.functions
			.get(name)
			.map(|f| f.effects)
			.or_else(|| self.externals.get(name).map(|e| e.effects))
			.or_else(|| trusted_external(name).map(|e| e.effects))
	}

	/// The events of each function as the source program emits them (`program` as written, before the passes lower its
	/// emits and handler blocks to calls), the constraints checked again with them
	pub fn with_events_of(mut self, program: &Node) -> Self {
		let events = event_effects(program);
		for (name, function) in self.functions.iter_mut() {
			let (own, all) = events.get(name).cloned().unwrap_or_default();
			function.own_events = own;
			function.events = all;
		}
		self.violations = self.check_constraints();
		self
	}

	pub fn entry_effects(&self) -> EffectSet {
		self.functions[ENTRY].effects
	}

	/// Whether the module must import from this capability's module
	pub fn needs(&self, capability: Capability) -> bool {
		self.externals.values().any(|external| external.capability == capability)
	}

	/// First capability the module needs beyond `granted`, with the external that needs it
	pub fn denied(&self, granted: &[Capability]) -> Option<(String, Capability)> {
		// `try f(x) else …` calls the program's own function through the host (guarded_call): nothing from outside
		self.externals
			.iter()
			.filter(|(name, _)| name.as_str() != crate::host::GUARDED_CALL)
			.find(|(_, external)| !granted.contains(&external.capability))
			.map(|(name, external)| (name.clone(), external.capability))
	}

	pub fn calls_external(&self, name: &str) -> bool {
		self.externals.contains_key(name)
	}

	/// Shortest call path from `from` to the external or function body that introduces `effect`
	pub fn call_chain(&self, from: &str, effect: Effect) -> Vec<String> {
		self.chain_to(from, |function| function.own.contains(effect), |external| external.effects.contains(effect))
	}

	/// Shortest call path from `from` to the function body (`in_body`) or external that introduces an effect
	fn chain_to(
		&self,
		from: &str,
		in_body: impl Fn(&FunctionEffects) -> bool,
		in_external: impl Fn(&External) -> bool,
	) -> Vec<String> {
		let mut predecessor: BTreeMap<String, String> = BTreeMap::new();
		let mut queue = VecDeque::from([from.to_string()]);
		while let Some(current) = queue.pop_front() {
			let introduces = self.externals.get(&current).is_some_and(&in_external)
				|| self.functions.get(&current).is_some_and(&in_body);
			if introduces {
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

	/// Diagnostic for the first violated constraint, else the answer to a trailing `effects of f` of a program whose top
	/// level has no effect: nothing to run, so the program needs no module (its functions may not even compile yet)
	pub fn answer(&self, program: &Node) -> Option<Node> {
		if let Some(violation) = self.violations.first() {
			return Some(Node::Error(Box::new(Node::Text(violation.to_string()))));
		}
		if let Some((name, constraint)) = self.misplaced.first() {
			return Some(Node::Error(Box::new(Node::Text(format!(
				"misplaced effect constraint at {}:{}: {name} is not a function, `! {}` needs a function definition",
				constraint.line, constraint.column, constraint.allowed
			)))));
		}
		let name = effects_query(last_statement(program))?;
		// the query mentions f, which reads as a call of it: the top level without the query
		EffectReport::of(&statements_before_last(program)).entry_effects().is_pure().then(|| self.query_answer(&name))
	}

	/// Every `effects of f` in the program as its constant answer (`IO`, `(IO ask)`), so it is a value anywhere: assigned,
	/// printed, the program's result
	pub fn answer_queries(&self, program: Node) -> Node {
		match effects_query(&program) {
			Some(name) => quoted(self.query_answer(&name)),
			None => program.map_children(|child| self.answer_queries(child)),
		}
	}

	fn query_answer(&self, name: &str) -> Node {
		match (self.functions.get(name), self.effects_of(name)) {
			(Some(function), _) => effects_node(function.effects, &function.events),
			(None, Some(effects)) => effects_node(effects, &BTreeSet::new()),
			(None, None) => Node::Error(Box::new(Node::Text(format!("effects of {name}: unknown function")))),
		}
	}
}

// ── Termination of self-recursion ────────────────────────────────────────────────────────────────
// Total if one parameter p is a measure: every recursive call sits in a branch whose guard bounds p
// by a number literal (`p<2 ? … : f(p-1)` bounds p ≥ 2 below) and passes `p - k` (or `p + k` toward
// an upper bound) at p's position, k a positive literal. p then moves by at least k per call and the
// call is only reached inside the bound, so every call chain is finite. p must not be assigned in the body.
// Assumes finite numbers: a float NaN or ∞ argument can still run away (exact numbers are the default).
