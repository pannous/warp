//! User functions: extraction, parameter and return kinds from bodies and calls, closures, negated calls

use super::*;
use crate::lowering::nodes::children_rewritten;
use crate::words::{ANY_TYPE, FLOAT_WORD, LIST_WORD, NUMBER_WORD};

mod definitions;
pub(crate) use definitions::*;

/// The parameters and variables of a function body, typed
pub fn function_body_scope(
	params: &[Param],
	body: &Node,
	function_kinds: &FunctionKinds,
	globals: &HashMap<String, Local>,
	closure_variable_targets: &ClosureTargets,
) -> Scope {
	let mut scope = Scope::with_function_kinds(function_kinds.clone()) // shared: a cheap copy
		.with_closure_targets(closure_variable_targets.clone());
	scope.globals = globals.clone(); // `d = o; return d` of a declared global keeps the global's kind
	for param in params {
		scope.define_param(param, param_kind(param));
	}
	collect_variables(body, &mut scope);
	scope
}

/// A global started exact (`b = 0.5`) holds an f64 throughout once a function gives it one (`global b; b = b + random()`),
/// as a main-level assignment widens it (widen_to_float), and a Node once a function gives it a value of run-time kind
/// (`global g = 0.3; set() := { g = f(-9) }`, widen_to_node). True when one was widened.
pub fn widen_globals_by_functions<'a>(
	globals: &mut HashMap<String, Local>,
	functions: impl Iterator<Item = &'a UserFunctionDef>,
	function_kinds: &FunctionKinds,
	closure_variable_targets: &ClosureTargets,
) -> bool {
	let mut widened_any = false;
	for function in functions {
		let scope =
			function_body_scope(&function.params, &function.body, function_kinds, globals, closure_variable_targets);
		widened_any |= widen_globals(globals, &scope.globals);
	}
	widened_any
}

/// The known globals that `seen` holds wider: as a Node, or as an f64 where they were exact. True when one was widened.
fn widen_globals(globals: &mut HashMap<String, Local>, seen: &HashMap<String, Local>) -> bool {
	let wider =
		|kind: Kind, than: Kind| kind != than && (kind == Kind::Empty || kind == Kind::Float && than != Kind::Empty);
	let mut widened_any = false;
	for (name, global) in seen {
		if let Some(widened) = globals.get_mut(name).filter(|known| wider(global.kind, known.kind)) {
			widened.kind = global.kind;
			widened.type_node = global.type_node.clone();
			widened_any = true;
		}
	}
	widened_any
}

/// The kinds of the values `return a, b` gives back, position by position over every such return:
/// one kind when all agree, Float for Int mixed with Float, else a Node
pub(super) fn infer_tuple_kinds(body: &Node, scope: &Scope) -> Vec<Kind> {
	let Some(arity) = crate::tuples::tuple_arity(body) else {
		return vec![];
	};
	let mut kinds: Vec<Option<Kind>> = vec![None; arity];
	body.visit(&mut |node| {
		for (kind, value) in kinds.iter_mut().zip(crate::tuples::returned_values(node).unwrap_or_default()) {
			let value_kind = infer_type(value, scope);
			*kind = Some(match *kind {
				None => value_kind,
				Some(known) if known == value_kind => known,
				Some(known)
					if known.is_primitive()
						&& value_kind.is_primitive()
						&& (known.is_float() || value_kind.is_float()) =>
				{
					Kind::Float
				}
				Some(_) => Kind::Data,
			});
		}
	});
	kinds.into_iter().map(|kind| kind.unwrap_or(Kind::Data)).collect()
}

/// The kind a function body returns, its parameters and variables typed in `scope` (function_body_scope)
pub(super) fn infer_function_return_kind(body: &Node, scope: &Scope) -> Kind {
	if crate::tuples::tuple_arity(body).is_some() {
		return Kind::List; // used whole, a tuple function's values are packed into a list
	}
	// `return error "…"` is the failure path: it does not decide what the function returns
	let is_error = |value: &Node| {
		let value = match value.drop_meta() {
			Node::List(items, _, _) if items.len() == 2 && items[0].is_symbol("return") => &items[1],
			other => other,
		};
		crate::errors::run_time::returned_error_message(value).is_some() // `error("…")` or `raise …`
	};
	// `if c { return "text" }; …`: a returned Node makes the function return Nodes
	// (`return [dist, prev]`: a List when every returned Node is one)
	let mut returned_nodes: Vec<Kind> = vec![];
	let mut returned: Vec<Kind> = vec![];
	body.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			if items.len() == 2 && items[0].is_symbol("return") && !is_error(&items[1]) {
				// `return` alone returns ø: a Node, so the function's numbers are Nodes too
				let kind = if matches!(items[1].drop_meta(), Node::Empty) {
					Kind::Empty
				} else {
					infer_type(&items[1], scope)
				};
				returned.push(kind);
				if kind.is_ref() && !returned_nodes.contains(&kind) {
					returned_nodes.push(kind);
				}
			}
		}
	});
	// `{ print b }`: the words of one statement, whose value is print's, not b's
	let last = crate::lowering::event_signals::last_statement(body);
	let last = last.as_ref();
	// `f() := {}`, a handler `{{}}`: a body doing nothing returns ø
	// `f() = ø` too
	let empty_block = matches!(last.drop_meta(), Node::List(items, Bracket::Curly, _) if items.is_empty())
		|| matches!(last.drop_meta(), Node::Empty);
	let last_kind = match (is_error(last), returned.first()) {
		(false, None) if empty_block => Kind::Empty,
		(true, Some(_)) if returned.contains(&Kind::Float) && returned.iter().all(|kind| !kind.is_ref()) => Kind::Float,
		(true, Some(first)) if returned.iter().all(|kind| kind == first) => *first,
		(true, Some(_)) => Kind::Empty, // returns of different kinds: a Node of unknown kind
		// `while true { …; return left }` with left a float: a float function, whatever the loop is worth
		_ => match infer_type(last, scope) {
			// a loop the function returns from inside: the returns decide, the loop's own value (a number or ø) does not
			Kind::Data if is_while_do(last) && !returned.is_empty() => {
				if returned.contains(&Kind::Float) {
					Kind::Float
				} else {
					Kind::Int
				}
			}
			Kind::Int if returned.contains(&Kind::Float) => Kind::Float,
			kind => kind,
		},
	};
	match returned_nodes.as_slice() {
		// `if c { return "out" }; [1, 2]`: a list here, a text there, known only at run time
		nodes if last_kind.is_ref() && nodes.iter().any(|kind| *kind != last_kind) => Kind::Empty,
		_ if last_kind.is_ref() => last_kind,
		[] => last_kind,
		[Kind::Empty] => Kind::Empty,
		[Kind::List] => Kind::List,
		_ => Kind::Empty, // Nodes of different kinds (a text here, a list there): known only at run time
	}
}

/// `while c do body`, what every loop lowers to
fn is_while_do(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(condition, Op::Do, _) if matches!(condition.drop_meta(), Node::Key(_, Op::While, _)))
}

/// An unannotated parameter without a default, or with an Int one (`within = 0`), takes the kind its callers pass:
/// `f(1, w = 1e-9)` gives w a float
fn takes_passed_kind(param: &Param) -> bool {
	param.annotation.is_none()
		&& param.default.as_ref().is_none_or(|value| argument_literal_kind(value) == Some(Kind::Int))
}

/// A parameter takes its declared `x:type` kind, else the kind of its default value (a fresh value per call); otherwise Int
pub fn param_kind(param: &Param) -> Kind {
	if let Some(kind) = param.annotation.as_ref().and_then(annotated_kind) {
		return kind;
	}
	match param
		.default
		.as_ref()
		.map(|value| argument_literal_kind(value).unwrap_or_else(|| infer_type(value, &Scope::new())))
	{
		Some(kind @ (Kind::Float | Kind::Text | Kind::List)) => kind,
		// `s="a"`: a one-character text is parsed as a Codepoint; the parameter holds any text
		Some(Kind::Codepoint) => Kind::Text,
		_ => param.used_as.unwrap_or(Kind::Int),
	}
}

/// Every definition form (`f(x) := …`, `fn`, `def`, `function`) infers its parameter and return kinds alike
pub(super) fn user_function(name: &str, params: Vec<Param>, body: Node) -> UserFunctionDef {
	let params = with_usage_kinds(params, &body);
	let mut scope = function_body_scope(&params, &body, &Default::default(), &HashMap::new(), &Default::default());
	// `number sine(number x){ x = x % turn …` with a float turn: the `number` parameter holds a float throughout
	let mut params = params;
	let mut widened = false;
	for param in
		params.iter_mut().filter(|param| scope.locals.get(&param.name).is_some_and(|local| local.kind.is_float()))
	{
		if param.annotation.as_ref().is_some_and(|declared| declared.name() == NUMBER_WORD) {
			param.annotation = Some(Node::Symbol(FLOAT_WORD.to_string()));
			widened = true;
		}
	}
	if widened {
		scope = function_body_scope(&params, &body, &Default::default(), &HashMap::new(), &Default::default());
	}
	let return_kind = infer_function_return_kind(&body, &scope);
	UserFunctionDef {
		name: name.to_string(),
		params,
		doc: crate::lowering::doc_strings::doc_of(&body),
		body: Box::new(body),
		return_kind,
		tuple_kinds: vec![],
		func_index: None,
	}
}

/// Undeclared parameters that the body indexes (`xs#2`, `it[1]`) or reads as a map (`g.keys()`, `xs.has(x)`) take a list;
/// those handed to `cell_get(c)` and the other cell words are cells
pub(super) fn with_usage_kinds(params: Vec<Param>, body: &Node) -> Vec<Param> {
	let usages = Usages::of(body);
	params.into_iter().map(|param| usages.typed(param)).collect()
}

/// How a function's body uses its names: called, indexed, as a cell, joined to a text, given to foreign code, tested
/// for its kind
#[derive(Default)]
struct Usages {
	indexed: HashSet<String>,
	called: HashSet<String>,
	celled: HashSet<String>,
	texted: HashSet<String>,
	foreign: HashSet<String>,
	tested: HashSet<String>,
	aliases: Vec<(String, String)>,
}

impl Usages {
	fn of(body: &Node) -> Self {
		let mut usages = Usages::default();
		body.visit(&mut |node| usages.record(node));
		// `for x in xs` walks the copy `x·items = xs`: the source of a used sequence is one too
		while let Some((_, source)) = usages
			.aliases
			.iter()
			.find(|(alias, source)| usages.indexed.contains(alias) && !usages.indexed.contains(source))
		{
			usages.indexed.insert(source.clone());
		}
		usages
	}

	fn record(&mut self, node: &Node) {
		if let Some(name) = crate::closures::called_closure(node) {
			self.called.insert(name.to_string());
		}
		if let Node::List(items, Bracket::Round, _) = node {
			if let [word, map, ..] = items.as_slice() {
				self.record_word_call(word, map);
			}
		}
		if let Some(Node::Symbol(name)) = used_sequence(node).map(Node::drop_meta) {
			self.indexed.insert(name.clone());
		}
		if let Some(name) = joined_to_text(node) {
			self.texted.insert(name.to_string());
		}
		self.foreign.extend(given_to_foreign_code(node).map(str::to_string));
		self.tested.extend(kind_tested(node).map(str::to_string));
		// `xs = xs + [420]`, `xs += [420]` (what `xs.add(420)` lowers to): xs is a list, called or not
		if let Some(name) = joined_to_list(node) {
			self.indexed.insert(name.to_string());
		}
		if let Node::Key(list, Op::AddAssign, added) = node {
			if let (Node::Symbol(name), Node::List(_, Bracket::Square, _)) = (list.drop_meta(), added.drop_meta()) {
				self.indexed.insert(name.clone());
			}
		}
		// `x as list`: a sequence, its characters when a text comes in
		if let Node::Key(value, Op::As, target) = node {
			if let Node::Symbol(name) = value.drop_meta() {
				if crate::type_tests::canonical_spec_word(&target.drop_meta().name().to_lowercase()) == LIST_WORD {
					self.indexed.insert(name.clone());
				}
			}
		}
		if let Node::Key(alias, Op::Assign | Op::Define, source) = node {
			if let (Node::Symbol(alias), Node::Symbol(source)) = (alias.drop_meta(), source.drop_meta()) {
				self.aliases.push((alias.clone(), source.clone()));
			}
		}
	}

	/// `(word map …)`: a map word reads map as a map, a cell word as a cell
	fn record_word_call(&mut self, word: &Node, map: &Node) {
		let Node::Symbol(name) = map.drop_meta() else {
			return;
		};
		// `t.left = 3` lowers to `t = field_with(t, "left", 3)`: t is an object too
		let is_map_word = word.symbol_name().is_some_and(|call| {
			crate::library_words::MAP_WORD_FUNCTIONS.contains(&call) || call == crate::library_words::FIELD_WITH
		});
		if is_map_word {
			self.indexed.insert(name.clone());
		}
		if crate::wasm_emitter::functions::cells::is_cell_word(word) {
			self.celled.insert(name.clone());
		}
	}

	fn used_as(&self, name: &String) -> Option<Kind> {
		if self.called.contains(name) {
			Some(Kind::Function)
		} else if self.celled.contains(name) || self.foreign.contains(name) {
			Some(Kind::Data)
		} else if self.indexed.contains(name) {
			// a counted sequence joined to a text is a text; joined alone it may be any value (`"set " + value`)
			Some(if self.texted.contains(name) { Kind::Text } else { Kind::List })
		} else {
			None
		}
	}

	fn typed(&self, param: Param) -> Param {
		// `if x is list then … else x`: a parameter tested for its kind takes any value, as if written `x:any` (card recursive-sum)
		let takes_any = self.tested.contains(&param.name) && param.annotation.is_none() && param.default.is_none();
		let annotation = if takes_any { Some(Node::Symbol(ANY_TYPE.to_string())) } else { param.annotation };
		Param { used_as: self.used_as(&param.name), annotation, ..param }
	}
}

/// `is_type(x, "list")`, what `x is list` lowers to: the name whose kind is tested
fn kind_tested(node: &Node) -> Option<&str> {
	let Node::List(items, _, _) = node else { return None };
	match items.as_slice() {
		[word, tested, _] if word.is_symbol(crate::words::IS_TYPE) => match tested.drop_meta() {
			Node::Symbol(name) => Some(name),
			_ => None,
		},
		_ => None,
	}
}

/// The names a foreign call takes as its receiver or arguments (lowering/foreign/foreign_modules.rs): values of any kind, as
/// foreign code gives them (a warp function called back by JavaScript, card js-callbacks)
fn given_to_foreign_code(node: &Node) -> impl Iterator<Item = &str> {
	let given = match node {
		Node::List(items, Bracket::Round, _) if node.is_call_of(crate::host::FOREIGN_CALL) => match items.as_slice() {
			[_, _, receiver, _, _, Node::List(arguments, _, _)] => std::iter::once(receiver).chain(arguments).collect(),
			[_, _, receiver, ..] => vec![receiver],
			_ => vec![],
		},
		_ => vec![],
	};
	given.into_iter().filter_map(Node::symbol_name)
}

/// The name `+` joins to a text: `" " + t`, `t + "!"`, `n times " " + t`, `t[0 ..< n] + "…"`; a list never
/// concatenates with a text
fn joined_to_text(node: &Node) -> Option<&str> {
	joined_to(node, is_literal_text)
}

/// The name `+` joins to a list literal: `xs + [x]`, `[x] + xs`
fn joined_to_list(node: &Node) -> Option<&str> {
	joined_to(node, |node| matches!(node, Node::List(_, Bracket::Square, _)))
}

/// A text literal or a repeated one `"ab" * 3`
fn is_literal_text(node: &Node) -> bool {
	node.is_text_literal() || node.is_call_of(crate::warp_parser::TEXT_TIMES)
}

/// The name joined by `+` to a value that `is_joined` accepts
fn joined_to(node: &Node, is_joined: fn(&Node) -> bool) -> Option<&str> {
	fn joined_name(node: &Node) -> Option<&str> {
		match node {
			Node::Symbol(name) => Some(name),
			Node::List(items, _, _) if node.is_call_of(crate::words::SLICE) => {
				match items.get(1).map(Node::drop_meta) {
					Some(Node::Symbol(name)) => Some(name),
					_ => None,
				}
			}
			_ => None,
		}
	}
	let Node::Key(left, Op::Add, right) = node else {
		return None;
	};
	let (left, right) = (left.drop_meta(), right.drop_meta());
	match (is_joined(left), is_joined(right)) {
		(true, false) => joined_name(right),
		(false, true) => joined_name(left),
		_ => None,
	}
}

/// The sequence a node uses: indexed `xs#2`, counted `#xs`, `count xs`, `xs.length`
pub(super) fn used_sequence(node: &Node) -> Option<&Node> {
	let is_counting_word = |word: &Node| word.symbol_name().is_some_and(is_counting_word);
	match node {
		Node::Key(list, Op::Hash, counted) => Some(if list.is_nothing() { counted } else { list }),
		Node::Key(list, Op::Dot, property) if is_counting_word(property) => Some(list),
		Node::List(items, _, _) => match items.as_slice() {
			[word, counted] if is_counting_word(word) => Some(counted),
			_ => None,
		},
		_ => None,
	}
}

/// A parameter forwarded to another function's parameter takes its kind: `f(grid) { g(grid) }` with g indexing grid
pub(super) fn infer_forwarded_parameters(ctx: &mut Context) {
	loop {
		let mut forwarded: Vec<(String, usize, Kind)> = vec![];
		for function in ctx.user_functions.values() {
			function.body.visit(&mut |node| {
				let Node::List(items, _, _) = node else { return };
				// `closure_new(target, captured…)` passes the captured values to the target's first parameters
				let items = match crate::closures::as_closure_new(node) {
					Some(_) => &items[1..],
					None => &items[..],
				};
				let Some(Node::Symbol(callee)) = items.first().map(Node::drop_meta) else {
					return;
				};
				// a closure call takes Nodes (as in infer_parameters_from_calls): its helper's parameters name no kind
				if crate::closures::closure_call_arity(callee).is_some() {
					return;
				}
				let Some(called) = ctx.user_functions.get(callee) else {
					return;
				};
				for (argument, called_param) in items[1..].iter().zip(&called.params) {
					let Node::Symbol(name) = argument.drop_meta() else {
						continue;
					};
					let Some(index) = function.params.iter().position(|param| &param.name == name) else {
						continue;
					};
					let (param, kind) = (&function.params[index], param_kind(called_param));
					if kind != Kind::Int
						&& param_kind(param) == Kind::Int
						&& param.annotation.is_none()
						&& param.default.is_none()
					{
						forwarded.push((function.name.clone(), index, kind));
					}
				}
			});
		}
		if forwarded.is_empty() {
			return;
		}
		for (name, index, kind) in forwarded {
			ctx.user_functions.get_mut(&name).expect("collected from user functions").params[index].used_as =
				Some(kind);
		}
	}
}

/// The functions, imports and closure facts of a program; the same tree is analysed once (analysis_memo.rs, P91).
/// Recognizes patterns:
/// - `name(param) = body` → Key(List[name, param], Assign, body)
/// - `name := body` → Key(Symbol(name), Define, body) (uses implicit `it`)
pub fn extract_user_functions(ctx: &mut Context, node: &Node) {
	crate::compiler::analysis_memo::analysed(ctx, node, analyse_user_functions);
}

/// A fresh context holding the functions the program defines (extract_user_functions)
pub fn function_context(node: &Node) -> Context {
	let mut context = Context::new();
	extract_user_functions(&mut context, node);
	context
}

/// A fresh context holding only what the program defines (defined_functions), no inferred kinds: for the passes that
/// read names, parameters and bodies, which so cost no analysis of the program as it stands
pub fn function_definitions(node: &Node) -> Context {
	let mut context = Context::new();
	defined_functions(&mut context, node);
	context
}

/// The functions a program defines (nested ones lifted as `outer·inner`) and the closure helpers it calls, without the
/// kinds the inference gives their parameters and results: what a pass that needs names, parameters and bodies reads
pub fn defined_functions(ctx: &mut Context, node: &Node) {
	extract_user_functions_inner(ctx, node);
	crate::closures::register_closure_calls(ctx, node);
}

pub(super) fn analyse_user_functions(ctx: &mut Context, node: &Node) {
	defined_functions(ctx, node);
	infer_parameters_from_calls(ctx, node);
	infer_forwarded_parameters(ctx);
	infer_closure_parameters(ctx, node);
	infer_forwarded_parameters(ctx); // the kinds closures gave their parameters reach the functions that pass them
	let globals = declared_globals(node);
	ctx.field_kinds = program_field_kinds(node);
	let mut globals = with_closure_captures(ctx, node, globals);
	refine_return_kinds(ctx, &globals);
	if type_float_list_parameters(ctx, node) {
		refine_return_kinds(ctx, &globals);
	}
	let return_kinds: FunctionKinds = FunctionKinds::new(
		ctx.user_functions.iter().map(|(name, function)| (name.clone(), function.return_kind)).collect(),
	);
	// `global g = 0.3; g = f(-9)` with an f that returns a Node: only the return kinds tell that main's g holds one
	let widened_by_main =
		widen_globals(&mut globals, &declared_globals_in(Scope::with_function_kinds(return_kinds.clone()), node));
	if widen_globals_by_functions(
		&mut globals,
		ctx.user_functions.values(),
		&return_kinds,
		&ctx.closure_variable_targets,
	) | widened_by_main
	{
		refine_return_kinds(ctx, &globals);
	}
	// a widened parameter can make the arguments it passes on floats too, to functions and to the closures it calls
	// (`h(x, f) := f(x); h(cos(0.5), a => a * a)`): until nothing changes (each round widens one)
	let parameter_count: usize = ctx.user_functions.values().map(|function| function.params.len()).sum();
	for _ in 0..parameter_count {
		if !(widen_parameters(ctx, node, &globals) | infer_closure_parameters(ctx, node)) {
			break;
		}
		refine_return_kinds(ctx, &globals);
	}
	crate::closures::type_closure_calls(ctx);
}

/// The kinds the user functions are known to give: each one's return kind and each value of a tuple function
/// (`(h, o) = f()` reads o's kind as `f#1`)
fn known_function_kinds(ctx: &Context) -> HashMap<String, Kind> {
	ctx.user_functions
		.iter()
		.flat_map(|(name, function)| {
			let values = function
				.tuple_kinds
				.iter()
				.enumerate()
				.map(|(index, kind)| (crate::tuples::element_key(name, index), *kind));
			std::iter::once((name.clone(), function.return_kind)).chain(values)
		})
		.collect()
}

/// A parameter that the calls pass one kind other than Int takes that kind: `mul(v, 1.0 / length(v))` with length
/// returning a float, `print_tree(tree.left, prefix + "│ ")` passing a text. infer_parameters_from_calls knows only
/// literal arguments; this pass runs once the return kinds are known. It only changes undeclared Int parameters (and
/// those guessed a list that get only texts), and leaves a parameter alone when calls disagree. True when a parameter changed.
pub(super) fn widen_parameters(ctx: &mut Context, program: &Node, globals: &HashMap<String, Local>) -> bool {
	let mut function_kinds = known_function_kinds(ctx);
	function_kinds.extend(ctx.field_kinds.clone());
	let function_kinds = FunctionKinds::new(function_kinds);
	let mut passed: HashMap<(String, usize), HashSet<Kind>> = HashMap::new();
	let mut main =
		Scope::with_function_kinds(function_kinds.clone()).with_closure_targets(ctx.closure_variable_targets.clone());
	collect_variables(program, &mut main);
	collect_argument_kinds(program, &main, ctx, &mut passed);
	for function in ctx.user_functions.values() {
		let scope = function_body_scope(
			&function.params,
			&function.body,
			&function_kinds,
			globals,
			&ctx.closure_variable_targets,
		);
		collect_argument_kinds(&function.body, &scope, ctx, &mut passed);
	}
	let mut changed = false;
	for ((name, index), kinds) in passed {
		let param = &mut ctx.user_functions.get_mut(&name).expect("collected from known functions").params[index];
		// passed only values held as Nodes (a loop variable over a list parameter): a Node, no int, and no list guessed
		// from indexing (capitalize(w) for the elements of a function's list result)
		// passed a Node and else only ints (`rect(x, levels[i], …)` and `rect(x, 2, …)`): the Node may hold a float, which an
		// int parameter refused at run time ("not an int"), so it takes the Node
		let unknown = matches!(param.used_as, None | Some(Kind::List | Kind::Int));
		let only_nodes_and_ints = kinds.iter().all(|kind| matches!(kind, Kind::Empty | Kind::Int));
		// passed keys (a pair, quoted code `data (n * 1)`), maybe beside lists (`data (g(y, z))`), where indexing guessed
		// a list: a Node, which `c#1` indexes as it is; a list parameter would wrap the key into a list of one (card
		// alias-indexed-param) and fold its key tests to false (card code-hole-arm-state)
		let keys_and_lists =
			kinds.contains(&Kind::Key) && kinds.iter().all(|kind| matches!(kind, Kind::Key | Kind::List));
		let keys_for_a_guessed_list = keys_and_lists && param.used_as == Some(Kind::List);
		if (kinds.contains(&Kind::Empty) && only_nodes_and_ints && unknown || keys_for_a_guessed_list)
			&& takes_passed_kind(param)
		{
			param.used_as = Some(Kind::Empty);
			changed = true;
			continue;
		}
		// passed a list and a number or a Node (`fract(v)` and `fract(item)` of a run-time choice, broadcasting each_any):
		// a Node, a list wrapped no number into a list of one
		let list_and_values =
			kinds.contains(&Kind::List) && kinds.iter().any(|kind| matches!(kind, Kind::Empty | Kind::Float | Kind::Data));
		if list_and_values && matches!(param.used_as, None | Some(Kind::List | Kind::Int)) && takes_passed_kind(param) {
			param.used_as = Some(Kind::Empty);
			changed = true;
			continue;
		}
		let mut kinds: Vec<Kind> =
			kinds.into_iter().filter(|kind| *kind != Kind::Int && *kind != Kind::Empty).collect();
		// passed a number of run-time kind and a float (`turned(p.xy, p.z * 0.12 + time)` and `turned(q, time)`): a
		// number decided at run time takes both; an Int parameter refused the float
		if kinds.contains(&Kind::Data) && kinds.iter().all(|kind| matches!(kind, Kind::Data | Kind::Float)) {
			kinds = vec![Kind::Data];
		}
		// passed values of several kinds held as Nodes (`kind_of(parse("x"))` and `kind_of(parse("x + 1"))`, a symbol
		// and a key): a Node, whose kind the run time tells
		let several_nodes = kinds.len() > 1 && kinds.iter().all(|kind| kind.is_ref() && *kind != Kind::Data);
		if several_nodes && unknown && takes_passed_kind(param) {
			param.used_as = Some(Kind::Empty);
			changed = true;
			continue;
		}
		let [kind] = kinds.as_slice() else { continue };
		// a parameter the body counts or indexes is guessed a list, until the calls pass it only texts
		let guessed = matches!(param.used_as, None | Some(Kind::Int))
			|| (param.used_as == Some(Kind::List) && *kind == Kind::Text);
		if takes_passed_kind(param) && guessed {
			param.used_as = Some(*kind);
			changed = true;
		}
	}
	changed
}

/// The user functions a list calls and their arguments: `f(a, b)`, and a task start that runs f in another instance with
/// them: `task·go(f, a, b)` (lower_tasks) and `task_spawn_values("f·node", [a, b])`, which
/// calls the wrapper f·node with the list and through it f with the items (resolve_tasks)
pub(crate) fn called_functions(items: &[Node]) -> Vec<(String, Vec<&Node>)> {
	let Some(Node::Symbol(head)) = items.first().map(Node::drop_meta) else {
		return vec![];
	};
	let started = match items.get(1).map(Node::drop_meta) {
		Some(Node::Text(name) | Node::Symbol(name)) => Some(name.clone()),
		_ => None,
	};
	match head.as_str() {
		crate::declarations::TASK_GO => started.map(|name| (name, items[2..].iter().collect())).into_iter().collect(),
		crate::host::TASK_SPAWN_VALUES | crate::host::GUARDED_CALL => match (started, items.get(2)) {
			(Some(wrapper), Some(list)) => {
				let mut calls = vec![(wrapper.clone(), vec![list])];
				if let (Some(function), Node::List(arguments, _, _)) =
					(wrapper.strip_suffix(crate::declarations::NODE_WRAPPER_SUFFIX), list.drop_meta())
				{
					calls.push((function.to_string(), arguments.iter().collect()));
				}
				calls
			}
			_ => vec![],
		},
		name => vec![(name.to_string(), items[1..].iter().collect())],
	}
}

/// The kinds each call in `node` passes each parameter of a user function, not looking into nested definitions
pub(super) fn collect_argument_kinds(
	node: &Node,
	scope: &Scope,
	ctx: &Context,
	passed: &mut HashMap<(String, usize), HashSet<Kind>>,
) {
	if function_definition_body(node).is_some() {
		return;
	}
	match node.drop_meta() {
		Node::List(items, _, _) => {
			for (name, arguments) in called_functions(items) {
				if let Some(function) = ctx.user_functions.get(&name) {
					for (index, argument) in arguments.into_iter().enumerate().take(function.params.len()) {
						passed.entry((name.clone(), index)).or_default().insert(infer_type(argument, scope));
					}
				}
			}
			items.iter().for_each(|item| collect_argument_kinds(item, scope, ctx, passed));
		}
		Node::Key(left, _, right) => {
			collect_argument_kinds(left, scope, ctx, passed);
			collect_argument_kinds(right, scope, ctx, passed);
		}
		Node::Meta { node, .. } => collect_argument_kinds(node, scope, ctx, passed),
		_ => {}
	}
}

/// Free variables a function reads from main (or an enclosing function) keep their kinds for return-kind inference:
/// without them `if x > limit then limit else x` types `limit` as Symbol, the function as Text, and a numeric `+` of
/// two calls traps (g-rT0c). Closures need the same (`t = "!"; shout = s => s + t`). Kinds are merged only where no
/// declared global of that name exists.
pub(super) fn with_closure_captures(
	ctx: &Context,
	program: &Node,
	mut globals: HashMap<String, Local>,
) -> HashMap<String, Local> {
	// `xs = [w(), w()]` holds what w returns, as the emitter's capture globals do (card float-calls)
	let mut outer = Scope::with_function_kinds(known_function_kinds(ctx));
	collect_variables(program, &mut outer);
	let mut functions: Vec<&UserFunctionDef> = ctx.user_functions.values().collect();
	functions.sort_by(|a, b| a.name.cmp(&b.name));
	for function in functions {
		let enclosing = ctx.enclosing_functions.get(&function.name).and_then(|name| ctx.user_functions.get(name));
		let captured: Vec<(String, Kind)> = match enclosing {
			Some(enclosing) => {
				let mut enclosing_scope = Scope::with_function_kinds(known_function_kinds(ctx));
				for param in &enclosing.params {
					enclosing_scope.define_param(param, param_kind(param));
				}
				collect_variables(&enclosing.body, &mut enclosing_scope);
				let mut captured = captured_variables(function, &enclosing_scope);
				captured.extend(
					captured_variables(function, &outer)
						.into_iter()
						.filter(|(name, _)| enclosing_scope.lookup(name).is_none()),
				);
				captured
			}
			None => captured_variables(function, &outer),
		};
		// the binding itself, its declared or literal type with it (`k = {a: 10}`: `k.a` is an Int there too)
		for (name, kind) in captured {
			let binding = outer.lookup(&name).cloned().map(|local| Local { kind, ..local });
			globals.entry(name.clone()).or_insert_with(|| binding.unwrap_or_else(|| Local::new(0, name, kind)));
		}
	}
	globals
}

/// The program's `global` declarations with their kinds
pub(crate) fn declared_globals(program: &Node) -> HashMap<String, Local> {
	declared_globals_in(Scope::new(), program)
}

fn declared_globals_in(mut scope: Scope, program: &Node) -> HashMap<String, Local> {
	collect_variables(program, &mut scope);
	scope.globals
}

/// The program's own functions that take arguments
pub fn applicable_function_names(node: &Node) -> HashSet<String> {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, node);
	ctx.user_functions.into_iter().filter(|(_, function)| !function.params.is_empty()).map(|(name, _)| name).collect()
}

/// A lifted lambda is never called by name (closures.rs): its captured values take the kinds of the variables captured (a
/// parameter of the enclosing function, a literal variable), its parameters the kinds of the literal arguments of the closure
/// calls of its arity
/// True when a parameter got a kind
pub(super) fn infer_closure_parameters(ctx: &mut Context, program: &Node) -> bool {
	let variable_kinds = variable_kinds(program, ctx, true);
	let mut changed = false;
	while kinds_given(ctx, closure_parameter_kinds(ctx, program, &variable_kinds)) {
		changed = true;
	}
	changed
}

/// The kinds of the captured values and parameters of lifted lambdas, by lambda and index
fn closure_parameter_kinds(
	ctx: &Context,
	program: &Node,
	variable_kinds: &HashMap<String, Kind>,
) -> Vec<(String, usize, Kind)> {
	let mut inferred: Vec<(String, usize, Kind)> = vec![];
	// a closure call may call any lifted lambda of its arity: the kinds its arguments pass to each one
	let mut passed: HashMap<(String, usize), HashSet<Kind>> = HashMap::new();
	for (body, scope) in body_scopes(ctx, program) {
		let value_kind = |value: &Node| match value.drop_meta() {
			Node::Symbol(name) => scope.lookup(name).map(|local| local.kind).or(variable_kinds.get(name).copied()),
			// an element of a list whose elements are of no known kind is held as its Node (`xs#1` of [cos(0.5), 1])
			Node::Key(indexed, Op::Hash, _) => Some(inference::element_kind(indexed, &scope).unwrap_or(Kind::Empty)),
			// a cell's value is a Node of any kind (a signal's new value, lowering/signals/signal_values.rs)
			Node::List(items, _, _)
				if items
					.first()
					.is_some_and(|word| word.drop_meta().name() == crate::wasm_emitter::functions::cells::CELL_GET) =>
			{
				Some(Kind::Empty)
			}
			_ => argument_literal_kind(value),
		};
		body.visit(&mut |node| {
			if let Some((target, captured)) = crate::closures::as_closure_new(node) {
				inferred.extend(
					captured
						.iter()
						.enumerate()
						.filter_map(|(index, value)| Some((target.to_string(), index, value_kind(value)?))),
				);
			}
			passed_by_closure_call(ctx, node, &value_kind, &mut passed);
		});
	}
	inferred
		.extend(passed.into_iter().filter_map(|((target, index), kinds)| Some((target, index, passed_kind(&kinds)?))));
	inferred
}

/// The scope of each body, the program's and each function's, with the kinds of its parameters and variables:
/// `m = float(2.5)` captured by a closure is a float
fn body_scopes<'a>(ctx: &'a Context, program: &'a Node) -> Vec<(&'a Node, Scope)> {
	let function_kinds: FunctionKinds = FunctionKinds::new(
		ctx.user_functions.iter().map(|(name, function)| (name.clone(), function.return_kind)).collect(),
	);
	let kinds_of = |params: &[Param], body: &Node| -> Scope {
		function_body_scope(params, body, &function_kinds, &HashMap::new(), &ctx.closure_variable_targets)
	};
	std::iter::once((program, kinds_of(&[], program)))
		.chain(
			ctx.user_functions
				.values()
				.map(|function| (function.body.as_ref(), kinds_of(&function.params, &function.body))),
		)
		.collect()
}

/// The kinds a closure call passes to each lifted lambda of its arity
fn passed_by_closure_call(
	ctx: &Context,
	node: &Node,
	value_kind: &dyn Fn(&Node) -> Option<Kind>,
	passed: &mut HashMap<(String, usize), HashSet<Kind>>,
) {
	let Node::List(items, _, _) = node else { return };
	let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) else {
		return;
	};
	let Some(arity) = crate::closures::closure_call_arity(name) else {
		return;
	};
	for (target, captured) in ctx.closure_targets.iter().filter(|(target, captured)| {
		ctx.user_functions.get(target).is_some_and(|function| function.params.len() == captured + arity)
	}) {
		for (index, argument) in items[2..].iter().enumerate() {
			// an Int is certain only as a literal: an unknown variable reads as one too
			let kind = value_kind(argument)
				.filter(|kind| *kind != Kind::Int || argument_literal_kind(argument) == Some(Kind::Int));
			passed.entry((target.clone(), captured + index)).or_default().extend(kind);
		}
	}
}

/// Gives each untyped lambda parameter its inferred kind; whether one changed
fn kinds_given(ctx: &mut Context, inferred: Vec<(String, usize, Kind)>) -> bool {
	let mut changed = false;
	for (target, index, kind) in inferred {
		let Some(param) = ctx.user_functions.get_mut(&target).and_then(|function| function.params.get_mut(index))
		else {
			continue;
		};
		if kind != Kind::Int && takes_passed_kind(param) && param.used_as.is_none() {
			param.used_as = Some(kind);
			changed = true;
		}
	}
	changed
}

/// The kind of a lambda parameter passed `kinds` by closure calls: one kind, a Float for Ints and Floats, any value (a
/// Node) for others mixed (Z combinator: `x(x)` passes a function, `fact(10)` an Int to lambdas of one arity, card
/// y-combinator)
fn passed_kind(kinds: &HashSet<Kind>) -> Option<Kind> {
	let mut others = kinds.iter().filter(|kind| **kind != Kind::Int);
	match (others.next(), others.next()) {
		(None, _) => None,
		(Some(&Kind::Float), None) => Some(Kind::Float),
		(Some(&kind), None) if !kinds.contains(&Kind::Int) => Some(kind),
		_ => Some(Kind::Empty),
	}
}

/// `a List`, `an Int`
pub fn kind_with_article(kind: Kind) -> String {
	with_article(&format!("{kind:?}"))
}

/// `a photo`, `an Int`, `an image`
pub fn with_article(name: &str) -> String {
	let article = if name.starts_with(|first: char| "AEIOUaeiou".contains(first)) { "an" } else { "a" };
	format!("{article} {name}")
}

/// The kinds of variables only ever assigned literals of one kind: `s="abcd"; f(s)` passes a Text.
/// A parameter of the same name shadows the variable (wiki/Footguns.md "Parameter shadowing"), so it is not judged.
pub(crate) fn literal_variable_kinds(program: &Node, ctx: &Context) -> HashMap<String, Kind> {
	variable_kinds(program, ctx, false)
}

/// literal_variable_kinds where a value of unknown kind leaves the literal's kind standing: `s = 0.5 as float; s = g(s)`
/// keeps s a Float, which types the closure g takes (infer_closure_parameters)
pub(super) fn variable_kinds(program: &Node, ctx: &Context, unknown_keeps_kind: bool) -> HashMap<String, Kind> {
	let mut kinds: HashMap<String, Option<Kind>> = HashMap::new();
	// a parameter of the same name shadows the variable; not for closures, whose captured parameters bear its name
	for param in ctx.user_functions.values().flat_map(|function| &function.params).filter(|_| !unknown_keeps_kind) {
		kinds.insert(param.name.clone(), None);
	}
	let mut literal: HashSet<String> = HashSet::new();
	let kinds = agreeing_kinds(program, kinds, |name, value| {
		let kind = argument_literal_kind(value);
		if kind.is_none() && unknown_keeps_kind {
			return None;
		}
		literal.insert(name.to_string());
		Some(kind)
	});
	kinds
		.into_iter()
		.filter(|(name, _)| !unknown_keeps_kind || literal.contains(name))
		.filter_map(|(name, kind)| Some((name, kind?)))
		.collect()
}

/// The kind all assignments of each variable agree on, None where two disagree; `kind_of` an assignment's value, None
/// to skip it
pub(crate) fn agreeing_kinds(
	program: &Node,
	mut kinds: HashMap<String, Option<Kind>>,
	mut kind_of: impl FnMut(&str, &Node) -> Option<Option<Kind>>,
) -> HashMap<String, Option<Kind>> {
	program.visit(&mut |node| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = node {
			if let Node::Symbol(name) = target.drop_meta() {
				if let Some(kind) = kind_of(name, value) {
					kinds
						.entry(name.clone())
						.and_modify(|known| *known = known.filter(|known| Some(*known) == kind))
						.or_insert(kind);
				}
			}
		}
	});
	kinds
}

/// Variables assigned a decimal literal (`y = 2.5`, `y = 3.0`)
pub(super) fn decimal_variables(program: &Node) -> HashSet<String> {
	variables_assigned(program, is_decimal_literal)
}

/// `ok = x > 2`
fn bool_variables(program: &Node) -> HashSet<String> {
	variables_assigned(program, |value| crate::analyzer::is_boolean(value, &Scope::new()))
}

/// The variables assigned a value that `holds`
fn variables_assigned(program: &Node, holds: impl Fn(&Node) -> bool) -> HashSet<String> {
	let mut names = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = node {
			if let (Node::Symbol(name), true) = (target.drop_meta(), holds(value)) {
				names.insert(name.clone());
			}
		}
	});
	names
}

/// An undeclared parameter takes the kind of its arguments when every call agrees, over the kind its use suggests
/// (an indexed parameter is a List unless it is passed a Text); calls that disagree are a reported conflict.
/// Parameters that only get Int arguments (or none) keep their usage kind. Arguments that are neither literals nor
/// literal variables are not judged here: a wrong one is still refused at the call by the emitter.
pub(super) fn infer_parameters_from_calls(ctx: &mut Context, program: &Node) {
	let judge = ArgumentJudge::of(ctx, program);
	let mut argument_kinds: HashMap<(String, usize), Vec<Kind>> = HashMap::new();
	let mut given_bools: HashMap<(String, usize), bool> = HashMap::new();
	program.visit(&mut |node| {
		let Node::List(items, _, _) = node else { return };
		for (name, arguments) in called_functions(items) {
			// a closure call takes Nodes, or the numbers type_closure_calls gives both it and its entries
			if crate::closures::closure_call_arity(&name).is_some() {
				continue;
			}
			let Some(function) = ctx.user_functions.get(&name) else {
				continue;
			};
			for (index, argument) in arguments.into_iter().enumerate().take(function.params.len()) {
				let param = &function.params[index];
				// the head `show(b)` and a recursive `show(b)` pass the parameter itself: no say in its kind
				if !argument.is_symbol(&param.name) {
					*given_bools.entry((name.clone(), index)).or_insert(true) &= judge.is_bool(argument);
				}
				ctx.parameter_conflicts.extend(judge.digits_lost(argument, param));
				// `g("a")`: a one-character text is parsed as a Codepoint; the parameter holds any text, as for a default
				if let Some(kind) =
					judge.kind(argument).map(|kind| if kind == Kind::Codepoint { Kind::Text } else { kind })
				{
					let kinds = argument_kinds.entry((name.clone(), index)).or_default();
					if !kinds.contains(&kind) {
						kinds.push(kind);
					}
				}
			}
		}
	});
	type_bool_parameters(ctx, given_bools);
	add_assigned_kinds(ctx, &mut argument_kinds);
	type_parameters_by_arguments(ctx, argument_kinds);
}

/// What the arguments of a program's calls are: of a literal kind, bools, decimals
struct ArgumentJudge {
	variable_kinds: HashMap<String, Kind>,
	bool_variables: HashSet<String>,
	decimal_variables: HashSet<String>,
}

impl ArgumentJudge {
	fn of(ctx: &Context, program: &Node) -> ArgumentJudge {
		// a parameter of the same name shadows the variable (as in literal_variable_kinds)
		let mut decimal_variables = decimal_variables(program);
		ctx.user_functions.values().flat_map(|function| &function.params).for_each(|param| {
			decimal_variables.remove(&param.name);
		});
		ArgumentJudge {
			variable_kinds: literal_variable_kinds(program, ctx),
			bool_variables: bool_variables(program),
			decimal_variables,
		}
	}

	fn kind(&self, argument: &Node) -> Option<Kind> {
		argument_literal_kind(argument).or_else(|| match argument.drop_meta() {
			Node::Symbol(name) => self.variable_kinds.get(name).copied(),
			_ => None,
		})
	}

	/// is every argument a parameter is given a bool (`show(1<2)`, `show(ok)` of `ok = x > 2`)
	fn is_bool(&self, argument: &Node) -> bool {
		crate::analyzer::is_boolean(argument, &Scope::new())
			|| argument.symbol_name().is_some_and(|name| self.bool_variables.contains(name))
	}

	fn has_digits_an_int_loses(&self, argument: &Node) -> bool {
		is_decimal_literal(argument)
			|| self.kind(argument) == Some(Kind::Float)
			|| argument.symbol_name().is_some_and(|name| self.decimal_variables.contains(name))
	}

	/// A float passed to a declared int parameter loses digits: a compile error (P49); `real`, `exact` are Ints that
	/// may hold a ratio: only a whole type loses a decimal's digits
	fn digits_lost(&self, argument: &Node, param: &Param) -> Option<String> {
		let declared = param
			.annotation
			.as_ref()
			.map(|annotation| annotation.name())
			.filter(|declared| crate::analyzer::checks::refuses_decimals(declared))?;
		if !self.has_digits_an_int_loses(argument) {
			return None;
		}
		// a whole float serializes as `2`; its decimal point is what makes it no int
		let written = match argument.drop_meta() {
			Node::Number(crate::extensions::numbers::Number::Float(value)) if value.fract() == 0.0 => {
				format!("{value:.1}")
			}
			other => other.serialize(),
		};
		Some(format!("{written} is no {declared}: write {written} as int"))
	}
}

/// Given a bool at every call, an untyped parameter is one, as if written `b: bool`: it keeps printing yes/no
/// (card bool-loses: `show(b) := "got " + b; show(1<2)` gave "got 1")
fn type_bool_parameters(ctx: &mut Context, given_bools: HashMap<(String, usize), bool>) {
	for ((name, index), _) in given_bools.into_iter().filter(|(_, all_bools)| *all_bools) {
		let param =
			&mut ctx.user_functions.get_mut(&name).expect("call sites were collected from known functions").params
				[index];
		if param.annotation.is_none() && param.default.is_none() {
			param.annotation = Some(Node::Symbol(crate::analyzer::BOOL_TYPE.to_string()));
		}
	}
}

/// A value its body gives the parameter is one more kind it holds; with no call of known kind it starts as an Int
fn add_assigned_kinds(ctx: &Context, argument_kinds: &mut HashMap<(String, usize), Vec<Kind>>) {
	for (name, function) in &ctx.user_functions {
		for (index, param) in function.params.iter().enumerate() {
			let assigned = assigned_literal_kinds(&function.body, &param.name);
			if !assigned.is_empty() {
				let kinds = argument_kinds
					.entry((name.clone(), index))
					.or_insert_with(|| vec![param.used_as.unwrap_or(Kind::Int)]);
				kinds.extend(assigned.into_iter().filter(|kind| !kinds.contains(kind)).collect::<Vec<_>>());
			}
		}
	}
}

/// The annotation an untyped list parameter takes when every call passes a list of floats
const FLOAT_LIST_TYPE: &str = "floats";
const FLOAT_ELEMENT: &str = "float";

/// An untyped list parameter that every call, all of them at the main level, passes a list of floats (`xs = [i * 0.1 as
/// float for i in 0..n]; sum(xs)`) takes `floats`, as if declared: its loop reads f64s instead of Nodes (card
/// untyped-parameters, ~6x faster). A call inside a function body passes values of that body's scope: no say, no type.
/// True when one was typed
fn type_float_list_parameters(ctx: &mut Context, program: &Node) -> bool {
	let called_in_bodies: HashSet<String> = ctx
		.user_functions
		.values()
		.flat_map(|function| function_calls(&function.body).into_iter().map(|(name, _)| name))
		.collect();
	let mut main = Scope::with_function_kinds(known_function_kinds(ctx))
		.with_closure_targets(ctx.closure_variable_targets.clone());
	collect_variables(program, &mut main);
	let mut element_types: HashMap<(String, usize), HashSet<String>> = HashMap::new();
	for (name, arguments) in function_calls(program).into_iter().filter(|(name, _)| !called_in_bodies.contains(name)) {
		let Some(function) = ctx.user_functions.get(&name) else {
			continue;
		};
		for (index, argument) in arguments.iter().enumerate().take(function.params.len()) {
			// the head `sum(values)` of the definition names the parameter itself: no call
			if argument.drop_meta().is_symbol(&function.params[index].name) {
				continue;
			}
			let list_type = crate::analyzer::list_type_name(argument, &main);
			let element = list_type.strip_prefix(crate::analyzer::LIST_OF_PREFIX).unwrap_or("");
			element_types.entry((name.clone(), index)).or_default().insert(element.to_string());
		}
	}
	let mut typed_any = false;
	for ((name, index), elements) in element_types {
		let Some(param) = ctx.user_functions.get_mut(&name).and_then(|function| function.params.get_mut(index)) else {
			continue;
		};
		let untyped =
			param.annotation.is_none() && param.default.is_none() && matches!(param.used_as, None | Some(Kind::List));
		if untyped && elements.len() == 1 && elements.contains(FLOAT_ELEMENT) {
			param.annotation = Some(Node::Symbol(FLOAT_LIST_TYPE.to_string()));
			param.used_as = Some(Kind::List);
			typed_any = true;
		}
	}
	typed_any
}

/// The calls of user-named functions within `node` and their arguments
fn function_calls(node: &Node) -> Vec<(String, Vec<Node>)> {
	let mut calls = vec![];
	node.visit(&mut |part| {
		if let Node::List(items, _, _) = part {
			calls.extend(
				called_functions(items)
					.into_iter()
					.map(|(name, arguments)| (name, arguments.into_iter().cloned().collect())),
			);
		}
	});
	calls
}

/// An untyped parameter given one kind holds it (Int stays the usage kind); given several kinds, any value
fn type_parameters_by_arguments(ctx: &mut Context, argument_kinds: HashMap<(String, usize), Vec<Kind>>) {
	for ((name, index), kinds) in argument_kinds {
		let function = ctx.user_functions.get_mut(&name).expect("call sites were collected from known functions");
		let param = &mut function.params[index];
		// `number sine(number x)` called with `π / 2` or `sqrt(2)`: a number parameter given a float holds floats, as
		// fit() widens an Int to a Float
		if param.annotation.as_ref().is_some_and(|declared| declared.name() == NUMBER_WORD)
			&& kinds.contains(&Kind::Float)
		{
			param.annotation = Some(Node::Symbol(FLOAT_WORD.to_string()));
			continue;
		}
		if param.annotation.is_some() || param.default.is_some() {
			continue;
		}
		match kinds.as_slice() {
			[Kind::Int] => {}
			[kind] => param.used_as = Some(*kind),
			// called with several kinds, it takes any value, as if written `x:any` (P173: silently, no annotation asked)
			[_, _, ..] => param.annotation = Some(Node::Symbol(ANY_TYPE.to_string())),
			[] => {}
		}
	}
}

/// The kinds of the literals `body` assigns to `name` (`n = "x"`), a character as a text
fn assigned_literal_kinds(body: &Node, name: &str) -> Vec<Kind> {
	let mut kinds = vec![];
	body.visit(&mut |node| {
		let Node::Key(target, Op::Assign, value) = node else {
			return;
		};
		if !target.is_symbol(name) {
			return;
		}
		if let Some(kind) =
			argument_literal_kind(value).map(|kind| if kind == Kind::Codepoint { Kind::Text } else { kind })
		{
			if !kinds.contains(&kind) {
				kinds.push(kind);
			}
		}
	});
	kinds
}

/// Infer every return kind again knowing all user functions (they shadow FFI names, recursion assumes Int first),
/// until the kinds settle: a call of a float-returning function is itself Float. After the first round only the
/// functions naming one whose kinds changed are inferred again (a closure call's kind joins its targets', unnamed)
pub(super) fn refine_return_kinds(ctx: &mut Context, globals: &HashMap<String, Local>) {
	// a wasm module's export gives what its header says (`f(t) := shout(t)` gives shout's char *, a text)
	let mut function_kinds: HashMap<String, Kind> = ctx
		.ffi_imports
		.iter()
		.filter(|(_, import)| crate::modules::wasm_modules::is_module_path(import.library))
		.map(|(name, import)| (name.clone(), crate::analyzer::signature_kind(import)))
		.collect();
	function_kinds.extend(ctx.user_functions.keys().map(|name| (name.clone(), Kind::Int)));
	function_kinds.extend(ctx.field_kinds.clone());
	let mut stale: Vec<&UserFunctionDef> = ctx.user_functions.values().collect();
	for _ in 0..=ctx.user_functions.len() {
		let known = FunctionKinds::new(function_kinds.clone());
		let inferred: Vec<(String, Kind)> = stale
			.iter()
			.map(|function| (function.name.clone(), settled_return_kind(function, ctx, &known, globals)))
			.collect();
		let tuples: Vec<(String, String, Kind)> =
			stale.iter().flat_map(|function| element_kinds(function, ctx, &known, globals)).collect();
		let changed: HashSet<String> = inferred
			.iter()
			.map(|(name, kind)| (name, name, kind))
			.chain(tuples.iter().map(|(function, key, kind)| (function, key, kind)))
			.filter(|(_, key, kind)| function_kinds.get(key.as_str()) != Some(kind))
			.map(|(function, _, _)| function.clone())
			.collect();
		if changed.is_empty() {
			break;
		}
		stale = naming_changed(ctx, &known, &changed);
		function_kinds.extend(inferred);
		function_kinds.extend(tuples.into_iter().map(|(_, key, kind)| (key, kind)));
	}
	for function in ctx.user_functions.values_mut() {
		function.return_kind = function_kinds[&function.name];
		function.tuple_kinds = (0..crate::tuples::tuple_arity(&function.body).unwrap_or(0))
			.map(|index| function_kinds[&crate::tuples::element_key(&function.name, index)])
			.collect();
	}
}

/// The kind function returns, given the kinds of the others; a closure call's kind joins its targets'
fn settled_return_kind(
	function: &UserFunctionDef,
	ctx: &Context,
	function_kinds: &FunctionKinds,
	globals: &HashMap<String, Local>,
) -> Kind {
	let returned =
		|kinds: &FunctionKinds| infer_function_return_kind(&function.body, &body_scope(function, ctx, kinds, globals));
	let kind = crate::closures::closure_call_kind(&function.name, ctx, function_kinds)
		.unwrap_or_else(|| returned(function_kinds));
	// `if t == ø { return [] }; return f(t.left) + [x]`: assumed to return an Int, the recursion makes an error of the
	// last value; assumed to return a Node of unknown kind, it settles
	if kind == Kind::Error && function_kinds.get(&function.name) == Some(&Kind::Int) {
		let mut unknown_recursion = HashMap::clone(function_kinds);
		unknown_recursion.insert(function.name.clone(), Kind::Empty);
		returned(&FunctionKinds::new(unknown_recursion))
	} else {
		kind
	}
}

/// The scope of a user function's body, given the kinds of the functions it calls
fn body_scope(
	function: &UserFunctionDef,
	ctx: &Context,
	function_kinds: &FunctionKinds,
	globals: &HashMap<String, Local>,
) -> Scope {
	function_body_scope(&function.params, &function.body, function_kinds, globals, &ctx.closure_variable_targets)
}

/// The kinds of the elements of the tuple function returns: (function, element key, kind)
fn element_kinds(
	function: &UserFunctionDef,
	ctx: &Context,
	function_kinds: &FunctionKinds,
	globals: &HashMap<String, Local>,
) -> Vec<(String, String, Kind)> {
	if crate::tuples::tuple_arity(&function.body).is_none() {
		return vec![];
	}
	infer_tuple_kinds(&function.body, &body_scope(function, ctx, function_kinds, globals))
		.into_iter()
		.enumerate()
		.map(|(index, kind)| (function.name.clone(), crate::tuples::element_key(&function.name, index), kind))
		.collect()
}

/// The functions to infer again: those naming a function whose kinds changed, and the closure calls
fn naming_changed<'a>(
	ctx: &'a Context,
	function_kinds: &FunctionKinds,
	changed: &HashSet<String>,
) -> Vec<&'a UserFunctionDef> {
	let changed: Vec<&str> = changed.iter().map(String::as_str).collect();
	ctx.user_functions
		.values()
		.filter(|function| {
			crate::closures::closure_call_kind(&function.name, ctx, function_kinds).is_some()
				|| function.body.mentions_any(&changed)
		})
		.collect()
}
