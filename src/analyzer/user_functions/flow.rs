//! The kinds of user function parameters and results as one flow inference (card src-analyzer, the Rust side of
//! lib/compiler/analyzer.warp): each undeclared parameter holds the join of the kinds that flow into it (the arguments
//! of every call, the values a closure captures, the literals its body assigns it), else the kind its body's use
//! suggests, else what the closure calls pass; results follow from the bodies, starting from Int. Every round
//! recomputes the results and the flows from the kinds of the last one, until nothing changes.

use super::*;

/// The kinds that flow into each parameter, by function and index
type Flows = HashMap<(String, usize), Vec<Kind>>;
/// The kind each parameter's use in its body suggests, taken before the flows
type Usages = HashMap<(String, usize), Option<Kind>>;

/// The flows of the program: those of its calls, closures and assignments, and the weak ones of the closure calls,
/// which may call any function value of their arity and so tell a parameter's kind only when all agree and nothing
/// else tells
#[derive(Default)]
struct ProgramFlows {
	strong: Flows,
	weak: Flows,
	/// the kinds of the parameters a parameter is passed on to: `osc(p, d) := osc(p, d, shape)` with a float d there
	forwarded: Flows,
}

/// Parameters, results and the globals functions widen, settled together
pub(super) fn settle(ctx: &mut Context, program: &Node, globals: &mut HashMap<String, Local>) {
	let usages: Usages = ctx
		.user_functions
		.iter()
		.flat_map(|(name, function)| {
			function.params.iter().enumerate().map(move |(index, param)| ((name.clone(), index), param.used_as))
		})
		.collect();
	let judge = ArgumentJudge::of(ctx, program);
	let slots: usize = ctx.user_functions.values().map(|function| function.params.len() + 1).sum();
	assume_int_results(ctx);
	let mut held = Flows::new();
	for _ in 0..=slots {
		let results_changed = step_result_kinds(ctx, &all_function_kinds(ctx), globals);
		let kinds = all_function_kinds(ctx);
		let flows = flows(ctx, program, &kinds, globals, &judge, &usages);
		// `global g = 0.3; g = f(-9)` with an f that returns a Node: only the results tell that main's g holds one
		let widened_by_main =
			widen_globals(globals, &declared_globals_in(Scope::with_function_kinds(kinds.clone()), program));
		let widened = widened_by_main
			| widen_globals_by_functions(globals, ctx.user_functions.values(), &kinds, &ctx.closure_variable_targets);
		if !(given_flows(ctx, &flows, &usages, &mut held) | widened | results_changed) {
			return;
		}
	}
}

/// Each undeclared parameter its flows joined, else its usage kind, else what the closure calls agree on; whether one
/// changed. A parameter turning back to a kind it `held` before cycles (`out(signal) := sound(clamped_samples(signal))`
/// feeding a list and a Node back and forth): it settles on all of them joined.
fn given_flows(ctx: &mut Context, flows: &ProgramFlows, usages: &Usages, held: &mut Flows) -> bool {
	let mut changed = false;
	// a closure call's helper takes Nodes, or the numbers type_closure_calls gives both it and its entries
	let functions =
		ctx.user_functions.iter_mut().filter(|(name, _)| crate::closures::closure_call_arity(name).is_none());
	for (name, function) in functions {
		for (index, param) in function.params.iter_mut().enumerate() {
			let key = (name.clone(), index);
			let passed = flows.strong.get(&key).and_then(|kinds| joined_known(kinds));
			// `number sine(number x)` called with `π / 2`: a number parameter given a float holds floats
			if passed == Some(Kind::Float)
				&& param.annotation.as_ref().is_some_and(|declared| declared.name() == NUMBER_WORD)
			{
				param.annotation = Some(Node::Symbol(FLOAT_WORD.to_string()));
				changed = true;
			}
			if !takes_passed_kind(param) {
				continue;
			}
			let agreed = || flows.weak.get(&key).and_then(|kinds| joined(kinds));
			let usage = || usages[&key].or_else(|| flows.forwarded.get(&key).and_then(|kinds| joined(kinds)));
			// given only ints, it keeps what its use suggests (a list for `xs#1`); given nothing else, the Nodes closure
			// calls pass (`go xs.map(x => f(x))` maps the elements as Nodes) make it any value
			let kind = match passed {
				Some(Kind::Int) => usage().or_else(|| agreed().filter(|kind| *kind != Kind::Empty)),
				None => usage().or_else(agreed),
				// passed keys (quoted code `data (n * 1)`) where indexing guessed a list: a Node, which `c#1` indexes as
				// it is (card alias-indexed-param)
				Some(Kind::Key) if usages[&key] == Some(Kind::List) => Some(Kind::Empty),
				passed => passed,
			};
			if param.used_as == kind {
				continue;
			}
			let history = held.entry(key).or_default();
			history.extend(param.used_as);
			let kind = match kind {
				Some(cycled) if history.contains(&cycled) => joined(history),
				kind => kind,
			};
			if param.used_as != kind {
				param.used_as = kind;
				changed = true;
			}
		}
	}
	changed
}

/// The kind all of `kinds` fit: one kind, a float for ints and floats, a text for texts and characters, a number decided
/// at run time for it and numbers, else any value (a Node)
fn joined(kinds: &[Kind]) -> Option<Kind> {
	kinds.iter().map(|kind| if *kind == Kind::Codepoint { Kind::Text } else { *kind }).reduce(|a, b| match (a, b) {
		_ if a == b => a,
		(Kind::Int, Kind::Float) | (Kind::Float, Kind::Int) => Kind::Float,
		(Kind::Data, Kind::Int | Kind::Float) | (Kind::Int | Kind::Float, Kind::Data) => Kind::Data,
		_ => Kind::Empty,
	})
}

/// The kinds joined, a Node of unknown kind beside known kinds leaving them standing (`added(xs, ys)` given a list and
/// a map value); beside numbers it stands, as a number parameter can't hold it (`rect(x, y, w, shown[band], c)` and
/// `rect(x, y, w, 2, c)`)
fn joined_known(kinds: &[Kind]) -> Option<Kind> {
	let floats = kinds.contains(&Kind::Float);
	let numbers = floats || kinds.contains(&Kind::Int);
	let known: Vec<Kind> = kinds
		.iter()
		.copied()
		.filter(|kind| (*kind != Kind::Empty || numbers) && !(floats && *kind == Kind::Data))
		.collect();
	joined(if known.is_empty() { kinds } else { &known })
}

/// The kinds every call, closure and assignment of the program passes the parameters, in the scopes of the last round
fn flows(
	ctx: &Context,
	program: &Node,
	kinds: &FunctionKinds,
	globals: &HashMap<String, Local>,
	judge: &ArgumentJudge,
	usages: &Usages,
) -> ProgramFlows {
	let mut flows = ProgramFlows::default();
	let mut main = Scope::with_function_kinds(kinds.clone()).with_closure_targets(ctx.closure_variable_targets.clone());
	collect_variables(program, &mut main);
	// the names something binds: a parameter, a global, a variable assigned anywhere (a lowered loop's item)
	let bound: HashSet<String> = (ctx.user_functions.values().flat_map(|function| &function.params))
		.map(|param| param.name.clone())
		.chain(globals.keys().cloned())
		.chain(
			std::iter::once(program)
				.chain(ctx.user_functions.values().map(|function| function.body.as_ref()))
				.flat_map(|body| body.assigned_names::<Vec<String>>(|op| matches!(op, Op::Assign | Op::Define))),
		)
		.collect();
	let bodies = ctx.user_functions.iter().map(|(name, function)| {
		let scope =
			function_body_scope(&function.params, &function.body, kinds, globals, &ctx.closure_variable_targets);
		// a parameter holding only what its use suggests passes that guess on weakly: `in(u) := to(u)…` of a method
		// guessed a list must not outvote the texts every other call passes `to`
		let guessed: HashSet<&str> = (function.params.iter().enumerate())
			.filter(|(index, param)| param.used_as.is_some() && param.used_as == usages[&(name.clone(), *index)])
			.map(|(_, param)| param.name.as_str())
			.collect();
		(function.body.as_ref(), scope, guessed, Some((name, function)))
	});
	for (body, scope, guessed, owner) in std::iter::once((program, main, HashSet::new(), None)).chain(bodies) {
		let is_free = |name: &str| !bound.contains(name);
		let told = |value: &Node| told_kind(value, &scope, judge, &is_free);
		visit_arguments(body, ctx, &mut |name, index, argument| {
			if let Some((owner, function)) = owner {
				let own = argument
					.drop_meta()
					.symbol_name()
					.and_then(|passed| function.params.iter().position(|param| param.name == passed));
				let kind = param_kind(&ctx.user_functions[name].params[index]);
				if let Some(own) = own.filter(|_| kind != Kind::Int) {
					flows.forwarded.entry((owner.clone(), own)).or_default().push(kind);
				}
			}
			let guess = argument.drop_meta().symbol_name().is_some_and(|name| guessed.contains(name));
			let flowing = if guess { &mut flows.weak } else { &mut flows.strong };
			flowing.entry((name.to_string(), index)).or_default().extend(told(argument));
		});
		body.visit(&mut |node| closure_flows(ctx, node, &told, &mut flows));
	}
	for (name, function) in &ctx.user_functions {
		for (index, param) in function.params.iter().enumerate() {
			let assigned = assigned_literal_kinds(&function.body, &param.name);
			flows.strong.entry((name.clone(), index)).or_default().extend(assigned);
		}
	}
	flows
}

/// The kind `value` passes on, if it tells one: an Int is what a value of unknown kind reads as, so only a literal one,
/// or a literal variable's, tells; a name the scope doesn't bind reads as a symbol, which tells only of a free name
/// (`f ↯`, `weekend(Sunday)`, `g int`), not of a parameter or global out of the scope's sight; nor does a number
/// of run-time kind without a told number in it (`unique(a + b)` of any values)
fn told_kind(value: &Node, scope: &Scope, judge: &ArgumentJudge, is_free: &dyn Fn(&str) -> bool) -> Option<Kind> {
	// a literal says what it is (`1s` a float), whatever its scope reads it as
	if let Some(kind) = judge.kind(value) {
		return Some(kind);
	}
	let kind = infer_type(value, scope);
	let tells = match value.drop_meta() {
		Node::Symbol(name) if kind == Kind::Symbol => is_free(name),
		Node::Key(left, _, right) if kind == Kind::Data => [left, right].iter().any(|side| {
			told_kind(side, scope, judge, is_free)
				.is_some_and(|kind| kind.is_int() || kind.is_float() || kind == Kind::Data)
		}),
		_ => kind != Kind::Int,
	};
	tells.then_some(kind)
}

/// `closure_new(target, captured…)` passes the captured values to the target's first parameters; a closure call
/// `closure_call_2(f, a, b)` passes its arguments weakly to every function value of its arity
fn closure_flows(ctx: &Context, node: &Node, told: &dyn Fn(&Node) -> Option<Kind>, flows: &mut ProgramFlows) {
	if let Some((target, captured)) = crate::closures::as_closure_new(node) {
		for (index, value) in captured.iter().enumerate() {
			flows.strong.entry((target.to_string(), index)).or_default().extend(told(value));
		}
	}
	let Node::List(items, _, _) = node else { return };
	let Some(arity) = items.first().and_then(Node::symbol_name).and_then(crate::closures::closure_call_arity) else {
		return;
	};
	for (target, captured) in &ctx.closure_targets {
		if ctx.user_functions.get(target).is_some_and(|function| function.params.len() == captured + arity) {
			for (index, argument) in items.iter().skip(2).enumerate() {
				flows.weak.entry((target.clone(), captured + index)).or_default().extend(told(argument));
			}
		}
	}
}
