//! Effect inference (DESIGN.md "Effects", "Effects as enforced capabilities").
//!
//! A function's effects are the union of the trusted signatures of everything it calls,
//! resolved to a fixpoint so recursion terminates. The resolved external calls also decide
//! which WASM imports a module declares: no IO call, no WASI import.
//! `Div` (as in Koka) marks code that may not terminate: a `while` loop, recursion that does not
//! demonstrably shrink a parameter toward a guarded base case, or mutual recursion. When unsure, Div.
//! Later the `EffectSet` moves onto semantic `Expr`/`FunctionDecl`; the report stays the query API.

use crate::analyzer::{extract_ffi_imports, extract_user_functions};
use crate::context::Context;
use crate::node::{Bracket, Node};
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
	("fetch_start", Host, &[IO]),
	("fetch_reply", Host, &[IO]),
	("read", Host, &[IO]),
	("warning", Host, &[IO]),
	("print", Wasi, &[IO]),
	("puts", Wasi, &[IO]),
	("puti", Wasi, &[IO]),
	("putl", Wasi, &[IO]),
	("putf", Wasi, &[IO]),
	("fd_write", Wasi, &[IO, Unsafe]),
	("execute", Sql, &[IO]),
	("exec", Process, &[IO]),
	// the standard library's adapters (src/std_adapters.rs): std_pure's words are pure like libm's, std_io's do IO
	("std_pure", Libm, &[]),
	("std_io", Host, &[IO]),
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
	/// May diverge: not provably terminating (Koka's `div`)
	Div,
	/// Runs a block known only at run time (`interpret e`, host run_block): its code is not known to the compiler
	Eval,
}

impl Effect {
	pub const ALL: [Effect; 8] = [State, Allocation, IO, FFI, Async, Unsafe, Div, Eval];

	const fn bit(self) -> u8 {
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

	pub fn is_pure(self) -> bool {
		self.observable() == Self::PURE
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
	/// Modules of another runtime, `use python "math"` (foreign_call, src/foreign.rs; user decision P89)
	Foreign,
}

impl Capability {
	pub const ALL: [Capability; 7] = [Host, Wasi, Ffi, Libm, Sql, Process, Foreign];
	/// Capabilities `eval` grants: all of them for now (user decision P88, "currently allow everything to everyone");
	/// the checks stay, a host that grants less will narrow this
	pub const GRANTED_BY_EVAL: [Capability; 7] = Capability::ALL;
	/// Capabilities untrusted code gets: all of them for now too (P88)
	pub const GRANTED_UNTRUSTED: [Capability; 7] = Capability::ALL;

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
	pub declared: Option<Constraint>,
	/// `effects` as `effects of f` and `! Pure` see them: without the State of the effect-handler plumbing
	/// (scoped_handlers.rs), which a block handler keeps to itself
	pub reported: EffectSet,
	/// The events the function emits that no enclosing block handler answers (card effect-handlers): named effects
	pub events: BTreeSet<String>,
	/// The events its body itself emits
	pub own_events: BTreeSet<String>,
}

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
		write!(f, "effect violation at {}:{}: {} is declared ! {} but performs {} via {}",
			self.constraint.line, self.constraint.column, self.function,
			effect_names(self.constraint.allowed, &self.constraint.events), effect_names(self.excess, &self.excess_events), self.chain.join(" → "))?;
		if self.excess.contains(Div) {
			write!(f, " (Div: may not terminate: a while loop, or recursion without a parameter that shrinks toward a guarded base case)")?;
		}
		Ok(())
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
		crate::analyzer::extract_host_words(&mut context, program);
		let resolver = Resolver { context: &context };

		let mut report = EffectReport::default();
		let (plumbing_globals, user_globals): (Vec<String>, Vec<String>) = main_level_globals(program).into_iter()
			.partition(|name| crate::scoped_handlers::is_plumbing_variable(name));
		for (name, definition) in &context.user_functions {
			let mut function = resolver.function(&definition.body);
			let parameters: Vec<&str> = definition.params.iter().map(|param| param.name.as_str()).collect();
			if reads_global(&definition.body, &user_globals, &parameters) {
				function.perform(State); // shared state read at call time (wiki/charged.md §3): never folded nor memoized
			} else if reads_global(&definition.body, &plumbing_globals, &parameters) {
				function.perform_unreported(State); // the active effect handler, which `effects of f` does not show
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
		report.externals = report.functions.values().flat_map(|f| f.calls.iter())
			.filter_map(|name| resolver.external(name).map(|external| (name.clone(), external)))
			.collect();
		report.infer_to_fixpoint();
		report.with_events_of(program)
	}

	/// A cycle through another function has no measure we can check: Div
	fn mark_mutual_recursion(&mut self) {
		let cyclic: Vec<String> = self.functions.iter()
			.filter(|(name, function)| function.calls.iter().any(|callee| callee != *name && self.reaches(callee, name)))
			.map(|(name, _)| name.clone()).collect();
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
				let inferred = self.functions[&name].calls.iter()
					.fold(self.functions[&name].effects, |all, callee| all.union(self.effects_of(callee).unwrap_or_default()));
				let reported = self.functions[&name].calls.iter()
					.fold(self.functions[&name].reported, |all, callee| all.union(self.reported_effects_of(callee)));
				let function = self.functions.get_mut(&name).expect("known function");
				changed |= function.effects != inferred || function.reported != reported;
				function.effects = inferred;
				function.reported = reported;
			}
			if !changed {
				return;
			}
		}
	}

	fn check_constraints(&self) -> Vec<EffectViolation> {
		self.functions.iter().filter_map(|(name, function)| {
			let constraint = function.declared.clone()?;
			let excess = function.reported.minus(constraint.allowed).observable();
			let excess_events: BTreeSet<String> = function.events.difference(&constraint.events).cloned().collect();
			let chain = match (excess.iter().next(), excess_events.iter().next()) {
				(Some(effect), _) => self.call_chain(name, effect),
				(None, Some(event)) => self.chain_to(name, |function| function.own_events.contains(event), |_| false),
				(None, None) => return None,
			};
			Some(EffectViolation { function: name.clone(), constraint, excess, excess_events, chain })
		}).collect()
	}

	/// Resolved effects of a user function, external, or `main`
	pub fn effects_of(&self, name: &str) -> Option<EffectSet> {
		self.functions.get(name).map(|f| f.effects)
			.or_else(|| self.externals.get(name).map(|e| e.effects))
			.or_else(|| trusted_external(name).map(|e| e.effects))
	}

	fn reported_effects_of(&self, name: &str) -> EffectSet {
		self.functions.get(name).map(|f| f.reported).or_else(|| self.effects_of(name)).unwrap_or_default()
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
		self.externals.iter().filter(|(name, _)| name.as_str() != crate::host::GUARDED_CALL).find(|(_, external)| !granted.contains(&external.capability))
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
	fn chain_to(&self, from: &str, in_body: impl Fn(&FunctionEffects) -> bool, in_external: impl Fn(&External) -> bool) -> Vec<String> {
		let mut predecessor: BTreeMap<String, String> = BTreeMap::new();
		let mut queue = VecDeque::from([from.to_string()]);
		while let Some(current) = queue.pop_front() {
			let introduces = self.externals.get(&current).is_some_and(&in_external) || self.functions.get(&current).is_some_and(&in_body);
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
		Some(match (self.functions.get(&name), self.effects_of(&name)) {
			(Some(function), _) => effects_node(function.reported, &function.events),
			(None, Some(effects)) => effects.to_node(),
			(None, None) => Node::Error(Box::new(Node::Text(format!("effects of {name}: unknown function")))),
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
		if name == crate::host::RUN_BLOCK {
			return Some(External { capability: Host, effects: EffectSet::of(&[Eval]) });
		}
		if name == crate::host::FOREIGN_CALL {
			return Some(External { capability: Foreign, effects: EffectSet::of(&[IO, FFI]) });
		}
		if let Some(import) = self.context.ffi_imports.get(name) {
			// libm is pure (P26): untrusted code and `! Pure` functions may call it; so are the module's own linear arrays
			let pure_library = import.library == crate::ffi::LIBM || import.library == crate::wasm_emitter::linear_arrays::LINEAR_LIBRARY;
			let task_word = import.library == crate::host::HOST_LIBRARY && crate::host::TASK_WORDS.contains(&name);
			return Some(match (pure_library, task_word) {
				(true, _) => External { capability: Libm, effects: EffectSet::PURE },
				(_, true) => External { capability: Ffi, effects: EffectSet::of(&[Async]) },
				_ => External { capability: Ffi, effects: EffectSet::of(&[FFI]) },
			});
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
			// `global effect_handler_active_ask`: the handler plumbing's own state (scoped_handlers.rs)
			_ if is_plumbing_declaration(node) => function.perform_unreported(State),
			Node::Symbol(name) if name == STATE_KEYWORD => function.perform(State),
			Node::Symbol(name) if self.resolves(name) => {
				function.calls.insert(name.clone());
			}
			// the arguments a lowering pass builds for a host word (`run_block xs#1 "xs" [xs]`) are no construction of
			// the program's own: the call keeps their other effects only
			Node::List(items, _, _) if items.first().is_some_and(|head| matches!(head.drop_meta(), Node::Symbol(name) if self.external(name).is_some())) => {
				let mut arguments = FunctionEffects::default();
				items.iter().for_each(|item| self.collect(item, &mut arguments));
				function.calls.extend(arguments.calls);
				function.own = function.own.union(arguments.own.observable());
				function.effects = function.effects.union(arguments.effects.observable());
			}
			Node::List(items, bracket, _) => {
				if constructs(bracket, items) {
					function.perform(Allocation);
				}
				items.iter().for_each(|item| self.collect(item, function))
			}
			Node::Key(left, op, right) => {
				if *op == Op::While && !never_true(right) {
					function.perform(Div);
				}
				if *op == Op::Add && [left, right].iter().any(|operand| matches!(operand.drop_meta(), Node::Text(_))) {
					function.perform(Allocation); // a text joined: "hi " + name
				}
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

impl FunctionEffects {
	fn perform(&mut self, effect: Effect) {
		self.perform_unreported(effect);
		self.reported = self.reported.union(EffectSet::of(&[effect]));
	}

	/// An effect the analysis keeps (nothing folds or memoizes the function) that `effects of f` does not show
	fn perform_unreported(&mut self, effect: Effect) {
		let effect = EffectSet::of(&[effect]);
		self.own = self.own.union(effect);
		self.effects = self.effects.union(effect);
	}
}

/// `global effect_handler_active_ask` (`global:…` once lowered): a declaration of handler plumbing variables only
fn is_plumbing_declaration(node: &Node) -> bool {
	let (keyword, variables): (&Node, Vec<&Node>) = match node.drop_meta() {
		Node::List(items, _, _) => match items.split_first() {
			Some((keyword, variables)) => (keyword, variables.iter().collect()),
			None => return false,
		},
		Node::Key(keyword, Op::Colon, variable) => (keyword, vec![variable]),
		_ => return false,
	};
	let names: Vec<String> = variables.into_iter().flat_map(crate::variable_signals::symbols).collect();
	keyword.drop_meta().name() == STATE_KEYWORD && !names.is_empty() && names.iter().all(|name| crate::scoped_handlers::is_plumbing_variable(name))
}

/// `Pure`, `ask`, `(IO ask)`: the effects and then the events
fn effects_node(effects: EffectSet, events: &BTreeSet<String>) -> Node {
	let mut names: Vec<Node> = effects.iter().map(|effect| format!("{effect:?}")).chain(events.iter().cloned()).map(Node::Symbol).collect();
	match names.len() {
		0 => Node::Symbol(PURE.into()),
		1 => names.remove(0),
		_ => Node::List(names, crate::node::Bracket::Round, crate::node::Separator::Space),
	}
}

/// `IO, ask`, `Pure`
fn effect_names(effects: EffectSet, events: &BTreeSet<String>) -> String {
	let names: Vec<String> = effects.iter().map(|effect| format!("{effect:?}")).chain(events.iter().cloned()).collect();
	if names.is_empty() { PURE.to_string() } else { names.join(", ") }
}

/// Each function's events, own and with its callees' (main for the top level): an `emit ask` adds ask, a call inside
/// `on ask {…} in {…}` passes on its callee's events but ask (card effect-handlers step 2)
pub fn event_effects(program: &Node) -> BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> {
	let mut context = Context::new();
	extract_user_functions(&mut context, program);
	let functions: BTreeSet<String> = context.user_functions.keys().cloned().collect();
	let verbs = crate::event_signals::emit_verbs(program);
	let mut uses: BTreeMap<String, EventUses> = context.user_functions.iter()
		.map(|(name, definition)| (name.clone(), EventUses::of(&definition.body, &functions, &verbs)))
		.collect();
	uses.insert(ENTRY.into(), EventUses::of(program, &functions, &verbs));
	let mut events: BTreeMap<String, BTreeSet<String>> = uses.iter().map(|(name, uses)| (name.clone(), uses.own.clone())).collect();
	loop {
		let mut changed = false;
		for (name, function) in &uses {
			let reached: BTreeSet<String> = function.calls.iter()
				.flat_map(|(callee, handled)| events.get(callee).into_iter().flatten().filter(|event| !handled.contains(*event)).cloned().collect::<Vec<_>>())
				.collect();
			let all = events.get_mut(name).expect("every function has its events");
			let before = all.len();
			all.extend(reached);
			changed |= all.len() != before;
		}
		if !changed {
			return uses.into_iter().map(|(name, function)| { let all = events.remove(&name).unwrap_or_default(); (name, (function.own, all)) }).collect();
		}
	}
}

/// The events a body emits itself and the calls it makes, each with the events the block handlers around it answer
#[derive(Default)]
struct EventUses {
	own: BTreeSet<String>,
	calls: Vec<(String, BTreeSet<String>)>,
}

impl EventUses {
	fn of(body: &Node, functions: &BTreeSet<String>, verbs: &[String]) -> Self {
		let mut uses = EventUses::default();
		uses.collect(body, &BTreeSet::new(), functions, verbs);
		uses
	}

	fn collect(&mut self, node: &Node, handled: &BTreeSet<String>, functions: &BTreeSet<String>, verbs: &[String]) {
		if defined_name(node).is_some_and(|name| functions.contains(&name)) {
			return; // a definition has its own events
		}
		if let Some((event, body, block)) = crate::scoped_handlers::scoped_handler(node) {
			self.collect(&body, handled, functions, verbs); // an emit in the handler reaches the next handler out
			let mut inside = handled.clone();
			inside.insert(event);
			return self.collect(&block, &inside, functions, verbs);
		}
		if let Some((event, data)) = crate::event_signals::emitted(node, verbs) {
			if !handled.contains(&event) {
				self.own.insert(event);
			}
			return self.collect(&data, handled, functions, verbs);
		}
		match node.drop_meta() {
			Node::Symbol(name) if functions.contains(name) => self.calls.push((name.clone(), handled.clone())),
			Node::List(items, _, _) => items.iter().for_each(|item| self.collect(item, handled, functions, verbs)),
			Node::Key(left, _, right) => {
				self.collect(left, handled, functions, verbs);
				self.collect(right, handled, functions, verbs);
			}
			_ => {}
		}
	}
}

/// A list `[a b]` or a record `{name: a}` built where it is written; not a block `{global y; y*y}`
fn constructs(bracket: &Bracket, items: &[Node]) -> bool {
	let is_field = |item: &Node| matches!(item.drop_meta(), Node::Key(key, Op::Colon, _) if key.drop_meta().name() != STATE_KEYWORD);
	match bracket {
		Bracket::Square => true,
		Bracket::Curly => !items.is_empty() && items.iter().all(is_field),
		_ => false,
	}
}

/// The variables the program declares `global` at its main level (`global limit = 10`)
fn main_level_globals(program: &Node) -> Vec<String> {
	let mut main = crate::analyzer::Scope::new();
	crate::analyzer::collect_variables(program, &mut main);
	main.globals.into_keys().collect()
}

/// Does a function body read one of `globals` that is not one of its parameters?
fn reads_global(body: &Node, globals: &[String], parameters: &[&str]) -> bool {
	let mut found = false;
	body.visit(&mut |node| found |= matches!(node, Node::Symbol(name) if globals.contains(name) && !parameters.contains(&name.as_str())));
	found
}

/// `while 0 {…}` / `while false {…}` never runs its body
fn never_true(condition: &Node) -> bool {
	match condition.drop_meta() {
		Node::Number(crate::extensions::numbers::Number::Int(0)) => true,
		Node::Symbol(word) => word == "false" || word == "False",
		Node::List(items, _, _) if items.len() == 1 => never_true(&items[0]),
		_ => false,
	}
}

// ── Termination of self-recursion ────────────────────────────────────────────────────────────────
// Total if one parameter p is a measure: every recursive call sits in a branch whose guard bounds p
// by a number literal (`p<2 ? … : f(p-1)` bounds p ≥ 2 below) and passes `p - k` (or `p + k` toward
// an upper bound) at p's position, k a positive literal. p then moves by at least k per call and the
// call is only reached inside the bound, so every call chain is finite. p must not be assigned in the body.
// Assumes finite numbers: a float NaN or ∞ argument can still run away (exact numbers are the default).

/// Which side of a literal a guard confines a parameter to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bound {
	/// p ≥ c or p > c: shrinking `p - k` terminates
	Lower,
	/// p ≤ c or p < c: growing `p + k` terminates
	Upper,
}

type Facts = Vec<(String, Bound)>;

fn recursion_shrinks(name: &str, parameters: &[&str], body: &Node) -> bool {
	let measures = parameters.iter().enumerate().filter(|(_, parameter)| !assigns(body, parameter));
	measures.flat_map(|(index, parameter)| [(index, *parameter, Bound::Lower), (index, *parameter, Bound::Upper)])
		.any(|(index, parameter, bound)| calls_shrink(body, name, index, parameter, bound, &mut vec![]))
}

/// Every occurrence of `name` in `node` is a call that moves argument `index` toward the known `bound` of `parameter`
fn calls_shrink(node: &Node, name: &str, index: usize, parameter: &str, bound: Bound, facts: &mut Facts) -> bool {
	let in_branch = |branch: &Node, guard: &Node, holds: bool, facts: &mut Facts| {
		let before = facts.len();
		facts.extend(guard_bounds(guard, holds));
		let shrinks = calls_shrink(branch, name, index, parameter, bound, facts);
		facts.truncate(before);
		shrinks
	};
	if let Some((guard, then, otherwise)) = conditional(node) {
		return calls_shrink(guard, name, index, parameter, bound, facts)
			&& in_branch(then, guard, true, facts)
			&& otherwise.is_none_or(|otherwise| in_branch(otherwise, guard, false, facts));
	}
	match node.drop_meta() {
		Node::Symbol(symbol) => symbol != name, // the function used as a value: unknown calls
		Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(callee)) if callee == name) => {
			let arguments = call_arguments(&items[1..]);
			let bounded = facts.iter().any(|(known, known_bound)| known == parameter && *known_bound == bound);
			bounded && arguments.get(index).is_some_and(|argument| moves_toward(argument, parameter, bound))
				&& arguments.iter().all(|argument| calls_shrink(argument, name, index, parameter, bound, facts))
		}
		Node::List(items, _, _) => items.iter().all(|item| calls_shrink(item, name, index, parameter, bound, facts)),
		Node::Key(left, _, right) => calls_shrink(left, name, index, parameter, bound, facts)
			&& calls_shrink(right, name, index, parameter, bound, facts),
		Node::Error(inner) => calls_shrink(inner, name, index, parameter, bound, facts),
		_ => true,
	}
}

/// `f(a, b)` and `f a b` both list their arguments after the name; `f((a))` unwraps one group
pub(crate) fn call_arguments(arguments: &[Node]) -> Vec<&Node> {
	match arguments {
		[single] => match single.drop_meta() {
			Node::List(items, crate::node::Bracket::Round, _) if !items.is_empty() => items.iter().collect(),
			Node::Empty => vec![],
			_ => vec![single],
		},
		_ => arguments.iter().collect(),
	}
}

/// (guard, then, else) of `c ? a : b`, `if c then a else b`, `if c {a}`
fn conditional(node: &Node) -> Option<(&Node, &Node, Option<&Node>)> {
	match node.drop_meta() {
		Node::Key(guard, Op::Question, branches) => match branches.drop_meta() {
			Node::Key(then, Op::Colon, otherwise) => Some((&**guard, &**then, Some(&**otherwise))),
			_ => Some((&**guard, &**branches, None)),
		},
		Node::Key(if_then, Op::Else, otherwise) => {
			let (guard, then, _) = if_then_parts(if_then)?;
			Some((guard, then, Some(&**otherwise)))
		}
		_ => if_then_parts(node),
	}
}

fn if_then_parts(node: &Node) -> Option<(&Node, &Node, Option<&Node>)> {
	let Node::Key(if_guard, Op::Then, then) = node.drop_meta() else { return None };
	match if_guard.drop_meta() {
		Node::Key(_, Op::If, guard) => Some((&**guard, &**then, None)),
		_ => None,
	}
}

/// Bounds on parameters known when `guard` evaluates to `holds`
fn guard_bounds(guard: &Node, holds: bool) -> Facts {
	match guard.drop_meta() {
		Node::List(items, _, _) if items.len() == 1 => guard_bounds(&items[0], holds),
		Node::Key(left, Op::And, right) if holds => [guard_bounds(left, true), guard_bounds(right, true)].concat(),
		Node::Key(left, Op::Or, right) if !holds => [guard_bounds(left, false), guard_bounds(right, false)].concat(),
		Node::Key(left, op @ (Op::Lt | Op::Le | Op::Gt | Op::Ge), right) => {
			let (parameter, below_literal) = match (left.drop_meta(), right.drop_meta()) {
				(Node::Symbol(parameter), Node::Number(limit)) if is_finite(limit) => (parameter, matches!(op, Op::Lt | Op::Le)),
				(Node::Number(limit), Node::Symbol(parameter)) if is_finite(limit) => (parameter, matches!(op, Op::Gt | Op::Ge)),
				_ => return vec![],
			};
			let bound = if below_literal == holds { Bound::Upper } else { Bound::Lower };
			vec![(parameter.clone(), bound)]
		}
		_ => vec![],
	}
}

/// `p - k` toward a lower bound, `p + k` toward an upper bound, k a positive literal
fn moves_toward(argument: &Node, parameter: &str, bound: Bound) -> bool {
	match argument.drop_meta() {
		Node::List(items, _, _) if items.len() == 1 => moves_toward(&items[0], parameter, bound),
		Node::Key(left, op @ (Op::Sub | Op::Add), step) => {
			let Node::Number(step) = step.drop_meta() else { return false };
			let Node::Symbol(moved) = left.drop_meta() else { return false };
			let direction = if *op == Op::Sub { -sign(step) } else { sign(step) };
			moved == parameter && direction == if bound == Bound::Lower { -1 } else { 1 }
		}
		_ => false,
	}
}

fn is_finite(number: &crate::extensions::numbers::Number) -> bool {
	use crate::extensions::numbers::Number;
	match number {
		Number::Int(_) | Number::Quotient(..) | Number::BigQuotient(_) | Number::BigInt(_) => true,
		Number::Float(value) => value.is_finite(),
		_ => false,
	}
}

/// Sign of a finite real literal, 0 when unknown
fn sign(number: &crate::extensions::numbers::Number) -> i8 {
	use crate::extensions::numbers::Number;
	let positive = match number {
		Number::Int(value) => *value > 0,
		Number::Float(value) => value.is_finite() && *value > 0.0,
		Number::Quotient(numerator, denominator) => (*numerator > 0) == (*denominator > 0) && *numerator != 0,
		Number::BigQuotient(q) => !q.is_zero() && !q.is_negative(),
		_ => return 0,
	};
	let negative = match number {
		Number::Int(value) => *value < 0,
		Number::Float(value) => value.is_finite() && *value < 0.0,
		Number::Quotient(numerator, denominator) => (*numerator > 0) != (*denominator > 0) && *numerator != 0,
		Number::BigQuotient(q) => q.is_negative(),
		_ => false,
	};
	if positive { 1 } else if negative { -1 } else { 0 }
}

/// The body writes `parameter` (`p = …`, `p += …`, `p++`): its value at a call is no longer the guarded one
fn assigns(node: &Node, parameter: &str) -> bool {
	match node.drop_meta() {
		Node::Key(left, op, right) => {
			let writes = matches!(op, Op::Assign | Op::Define | Op::AddAssign | Op::SubAssign | Op::MulAssign | Op::DivAssign
				| Op::ModAssign | Op::PowAssign | Op::AndAssign | Op::OrAssign | Op::XorAssign | Op::Inc | Op::Dec);
			(writes && matches!(left.drop_meta(), Node::Symbol(name) if name == parameter))
				|| (matches!(op, Op::Inc | Op::Dec) && matches!(right.drop_meta(), Node::Symbol(name) if name == parameter))
				|| assigns(left, parameter) || assigns(right, parameter)
		}
		Node::List(items, _, _) => items.iter().any(|item| assigns(item, parameter)),
		Node::Error(inner) => assigns(inner, parameter),
		_ => false,
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

fn constraint_names(node: &Node) -> Option<Vec<String>> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(vec![name.clone()]),
		Node::List(items, _, _) => items.iter().map(|item| constraint_names(item).and_then(|n| n.into_iter().next())).collect(),
		_ => None,
	}
}

fn is_effect_name(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if EffectSet::named(&[name]).is_some())
}

/// `definition ! Effects` → (definition, allowed effects)
fn split_constraint(node: &Node) -> Option<(&Node, (EffectSet, BTreeSet<String>))> {
	let Node::Key(definition, Op::Not, declared) = node.drop_meta() else { return None };
	defined_name(definition)?;
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

fn constraints(program: &Node) -> Vec<(String, Constraint)> {
	let mut found = vec![];
	collect_constraints(program, &mut found);
	found
}

fn collect_constraints(node: &Node, found: &mut Vec<(String, Constraint)>) {
	if let Some((definition, (allowed, events))) = split_constraint(node) {
		let (line, column) = node.get_lineinfo().map(|info| (info.line_nr, info.column)).unwrap_or_default();
		found.push((defined_name(definition).expect("checked by split_constraint"), Constraint { allowed, events, line, column }));
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
