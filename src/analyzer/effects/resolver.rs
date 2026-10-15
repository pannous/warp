//! Resolving calls to effects: the trusted signatures of external words and what each function body performs
use super::*;

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
	("wasi_environment", Wasi, &[IO]),
	("execute", Sql, &[IO]),
	("exec", Process, &[IO]),
	// the standard library's adapters (src/modules/std_adapters.rs): std_pure's words are pure like libm's, std_io's do IO;
	// a failed adapter gives back its Error, raised in the program (Fail)
	("std_pure", Libm, &[Fail]),
	("std_io", Host, &[IO, Fail]),
];

/// The operators that fail for a divisor of 0
const DIVISIONS: [Op; 5] = [Op::Div, Op::Mod, Op::Rem, Op::DivAssign, Op::ModAssign];
/// `try x else y` as parsed, then as lowered: the block that may stop at an error, the guarded call of a function
const TRY_HEADS: [&str; 3] =
	[crate::warp_parser::TRY_MARKER, crate::wasm_emitter::RAN_WITHOUT_ERROR, crate::host::GUARDED_CALL];

pub(super) struct Resolver<'a> {
	pub(super) context: &'a Context,
}

impl Resolver<'_> {
	pub(super) fn external(&self, name: &str) -> Option<External> {
		if self.context.user_functions.contains_key(name) {
			return None;
		}
		if name == crate::host::RUN_BLOCK {
			return Some(External { capability: Host, effects: EffectSet::of(&[Eval]) });
		}
		// the host calls the program's own function back (`try f(x) else …`): nothing from outside
		if name == crate::host::GUARDED_CALL {
			return Some(External { capability: Host, effects: EffectSet::PURE });
		}
		if name == crate::host::FOREIGN_CALL {
			// a crate's or component's Err is raised in the program
			return Some(External { capability: Foreign, effects: EffectSet::of(&[IO, FFI, Fail]) });
		}
		if let Some(import) = self.context.ffi_imports.get(name) {
			// libm is pure (P26): untrusted code and `! Pure` functions may call it; so are the module's own linear arrays
			let pure_library = import.library == crate::ffi::LIBM
				|| import.library == crate::wasm_emitter::lists::linear_arrays::LINEAR_LIBRARY;
			// warp's own host words (serve_routes, sleep, gpu_render …) are the Host capability, not C: a sandboxed program
			// (warp --sandbox) keeps them
			let host_word = import.library == crate::host::HOST_LIBRARY;
			let capability = if host_word { Host } else { Ffi };
			return Some(match (pure_library, host_word && crate::host::TASK_WORDS.contains(&name)) {
				(true, _) => External { capability: Libm, effects: EffectSet::PURE },
				(_, true) => External { capability, effects: EffectSet::of(&[Async]) },
				_ => External { capability, effects: EffectSet::of(&[FFI]) },
			});
		}
		trusted_external(name)
	}

	fn resolves(&self, name: &str) -> bool {
		self.context.user_functions.contains_key(name) || self.external(name).is_some()
	}

	pub(super) fn function(&self, body: &Node) -> FunctionEffects {
		let mut function = FunctionEffects::default();
		self.collect(body, &mut function);
		function
	}

	/// Every symbol naming a known function counts as a call; nested definitions and
	/// import declarations are skipped (definitions are analyzed on their own).
	pub(super) fn collect(&self, node: &Node, function: &mut FunctionEffects) {
		if self.is_definition(node) || is_import_statement(node) {
			return;
		}
		match node.drop_meta() {
			// a cell is shared: a `nonlocal` or a watched variable another function sees (signal_values)
			Node::Symbol(name) if name == STATE_KEYWORD || name == crate::wasm_emitter::functions::cells::CELL_SET => {
				function.perform(State)
			}
			// a closure call may run any lambda the program lifted: a listener closure's print is the writer's IO
			Node::Symbol(name) if name.starts_with(crate::closures::CLOSURE_CALL_PREFIX) => function.calls.extend(
				self.context
					.user_functions
					.keys()
					.filter(|lifted| lifted.starts_with(crate::closures::LIFTED_PREFIX))
					.cloned(),
			),
			Node::Symbol(name) if self.resolves(name) => {
				function.calls.insert(name.clone());
				function.tried_calls.remove(name);
			}
			// `try x else y` handles x's failure: its other effects stay
			Node::List(items, _, _)
				if items.first().is_some_and(|head| TRY_HEADS.iter().any(|word| head.is_symbol(word))) =>
			{
				let mut tried = FunctionEffects::default();
				items.iter().for_each(|item| self.collect(item, &mut tried));
				for callee in tried.calls {
					if function.calls.insert(callee.clone()) {
						function.tried_calls.insert(callee);
					}
				}
				function.own = function.own.union(tried.own.minus(FAILS));
				function.effects = function.effects.union(tried.effects.minus(FAILS));
			}
			Node::List(items, _, _) if raises(items) => {
				function.perform(Fail);
				items.iter().for_each(|item| self.collect(item, function))
			}
			// the arguments a lowering pass builds for a host word (`run_block xs#1 "xs" [xs]`) are no construction of
			// the program's own: the call keeps their other effects only
			Node::List(items, _, _)
				if items.first().is_some_and(
					|head| matches!(head.drop_meta(), Node::Symbol(name) if self.external(name).is_some()),
				) =>
			{
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
				if DIVISIONS.contains(op) && !is_nonzero_number(right) {
					function.perform(Fail); // 10/x fails for x = 0
				}
				self.collect(left, function);
				self.collect(right, function);
			}
			Node::Error(inner) => self.collect(inner, function),
			_ => {}
		}
	}

	fn is_definition(&self, node: &Node) -> bool {
		node.defined_name().is_some_and(|name| self.context.user_functions.contains_key(&name))
	}
}

impl FunctionEffects {
	pub(super) fn perform(&mut self, effect: Effect) {
		let effect = EffectSet::of(&[effect]);
		self.own = self.own.union(effect);
		self.effects = self.effects.union(effect);
	}
}

/// `raise "no"`, `throw e`, `fail`, `panic`, `error "neg"`, and the floor division `x // y` of a divisor that may be 0
fn raises(items: &[Node]) -> bool {
	match items {
		[word, _] => matches!(word.drop_meta(), Node::Symbol(word)
			if crate::errors::run_time::RAISE_WORDS.contains(&word.as_str()) || word == crate::words::ERROR_WORD),
		[word, _, divisor] => word.is_symbol(crate::warp_parser::FLOOR_QUOTIENT) && !is_nonzero_number(divisor),
		_ => false,
	}
}

fn is_nonzero_number(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Number(number) if f64::from(*number) != 0.0)
}

pub(super) fn trusted_external(name: &str) -> Option<External> {
	TRUSTED_EXTERNALS
		.iter()
		.find(|(known, _, _)| *known == name)
		.map(|(_, capability, effects)| External { capability: *capability, effects: EffectSet::of(effects) })
}

/// The variables the program declares `global` at its main level (`global limit = 10`)
pub(super) fn main_level_globals(program: &Node) -> Vec<String> {
	let mut main = crate::analyzer::Scope::new();
	crate::analyzer::collect_variables(program, &mut main);
	main.globals.into_keys().collect()
}

/// Does a function body read one of `globals` that is not one of its parameters?
pub(super) fn reads_global(body: &Node, globals: &[String], parameters: &[&str]) -> bool {
	body.mentions_where(|name| globals.iter().any(|global| global == name) && !parameters.contains(&name))
}

/// `import f from lib` / `use lib`
fn is_import_statement(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(items, _, _) => {
			matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if crate::modules::is_import_keyword(word))
		}
		_ => false,
	}
}
