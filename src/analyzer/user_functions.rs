//! User functions: extraction, parameter and return kinds from bodies and calls, closures, negated calls

use super::*;

/// Kind of a value known before running the program
pub(crate) fn literal_kind(value: &Node) -> Option<Kind> {
	match value.drop_meta() {
		Node::Number(Number::Int(_) | Number::BigInt(_)) | Node::True | Node::False => Some(Kind::Int),
		Node::Number(_) => Some(Kind::Float),
		Node::Text(_) => Some(Kind::Text),
		Node::Char(_) => Some(Kind::Codepoint),
		Node::Key(nothing, Op::Neg, operand) if matches!(nothing.drop_meta(), Node::Empty) => literal_kind(operand),
		_ => None,
	}
}

/// The kind of a literal or of arithmetic on literals: `π*1000000` is a float, `2*3` and `2.0*3` (exact decimals) Ints
pub(super) fn computed_literal_kind(value: &Node) -> Option<Kind> {
	match value.drop_meta() {
		Node::Key(left, op, right) if op.is_arithmetic() && !matches!(left.drop_meta(), Node::Empty) => {
			match (operand_kind(left)?, operand_kind(right)?) {
				// exact: whole or not by its value, `3.5*2` is 7, `3.3*2` is 6.6
				(Kind::Int, Kind::Int) => match exact_literal_value(value) {
					Some(exact) if !exact.is_integer() => Some(Kind::Float),
					_ => Some(Kind::Int),
				},
				(Kind::Float | Kind::Int, Kind::Float | Kind::Int) => Some(Kind::Float),
				_ => None,
			}
		}
		_ => literal_kind(value),
	}
}

/// The exact value of arithmetic on exact literals (`3.3*2` is 33/5), None for anything else or a division by zero
pub(super) fn exact_literal_value(node: &Node) -> Option<crate::extensions::reals::Rational> {
	match node.drop_meta() {
		Node::Number(Number::Float(f)) if Number::is_exact_decimal(*f) => {
			let (numerator, denominator) = crate::wasm_emitter::exact::decimal_fraction(*f);
			Some(crate::extensions::reals::Rational::new(numerator, denominator))
		}
		Node::Number(number @ (Number::Int(_) | Number::BigInt(_) | Number::Quotient(..) | Number::BigQuotient(_))) => Some(number.to_rational()),
		Node::Key(nothing, Op::Neg | Op::Sub, operand) if matches!(nothing.drop_meta(), Node::Empty) => Some(exact_literal_value(operand)?.neg()),
		Node::Key(left, op, right) => {
			let (left, right) = (exact_literal_value(left)?, exact_literal_value(right)?);
			match op {
				Op::Add => Some(left.add(&right)),
				Op::Sub => Some(left.add(&right.neg())),
				Op::Mul => Some(left.mul(&right)),
				Op::Div => Some(left.mul(&right.inverse()?)),
				_ => None,
			}
		}
		_ => None,
	}
}

/// An operand of arithmetic on literals: an exact decimal stays exact (`2.0*3` is 6), a real constant is a float
pub(super) fn operand_kind(operand: &Node) -> Option<Kind> {
	match operand.drop_meta() {
		Node::Symbol(name) if REAL_CONSTANTS.contains(&name.as_str()) => Some(Kind::Float),
		Node::Number(Number::Float(f)) if Number::is_exact_decimal(*f) => Some(Kind::Int),
		Node::Key(..) => computed_literal_kind(operand),
		_ => literal_kind(operand),
	}
}

/// Extract user-defined functions from the AST into context
/// Infer return type of a function body given its parameters
/// The parameters and variables of a function body, typed
pub(super) fn function_body_scope(params: &[Param], body: &Node, function_kinds: &HashMap<String, Kind>, globals: &HashMap<String, Local>, closure_variable_targets: &HashMap<String, HashSet<String>>) -> Scope {
	let mut scope = Scope::with_function_kinds(function_kinds.clone()).with_closure_targets(closure_variable_targets.clone());
	scope.globals = globals.clone(); // `d = o; return d` of a declared global keeps the global's kind
	for param in params {
		scope.define_param(param.name.clone(), param_kind(param));
	}
	collect_variables(body, &mut scope);
	scope
}

/// The kinds of the values `return a, b` gives back, position by position over every such return:
/// one kind when all agree, Float for Int mixed with Float, else a Node
pub(super) fn infer_tuple_kinds(params: &[Param], body: &Node, function_kinds: &HashMap<String, Kind>, globals: &HashMap<String, Local>, closure_variable_targets: &HashMap<String, HashSet<String>>) -> Vec<Kind> {
	let Some(arity) = crate::tuples::tuple_arity(body) else { return vec![] };
	let scope = function_body_scope(params, body, function_kinds, globals, closure_variable_targets);
	let mut kinds: Vec<Option<Kind>> = vec![None; arity];
	body.visit(&mut |node| {
		for (kind, value) in kinds.iter_mut().zip(crate::tuples::returned_values(node).unwrap_or_default()) {
			let value_kind = infer_type(value, &scope);
			*kind = Some(match *kind {
				None => value_kind,
				Some(known) if known == value_kind => known,
				Some(known) if known.is_primitive() && value_kind.is_primitive() && (known.is_float() || value_kind.is_float()) => Kind::Float,
				Some(_) => Kind::Data,
			});
		}
	});
	kinds.into_iter().map(|kind| kind.unwrap_or(Kind::Data)).collect()
}

pub(super) fn infer_function_return_kind(params: &[Param], body: &Node, function_kinds: &HashMap<String, Kind>, globals: &HashMap<String, Local>, closure_variable_targets: &HashMap<String, HashSet<String>>) -> Kind {
	if crate::tuples::tuple_arity(body).is_some() {
		return Kind::List; // used whole, a tuple function's values are packed into a list
	}
	let scope = function_body_scope(params, body, function_kinds, globals, closure_variable_targets);
	// `return error("…")` is the failure path: it does not decide what the function returns
	let is_error = |value: &Node| {
		let value = match value.drop_meta() {
			Node::List(items, _, _) if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(word) if word == "return") => &items[1],
			other => other,
		};
		crate::pipeline::returned_error_message(value).is_some() // `error("…")` or `raise …`
	};
	// `if c { return "text" }; …`: a returned Node makes the function return Nodes
	// (`return [dist, prev]`: a List when every returned Node is one)
	let mut returned_nodes: Vec<Kind> = vec![];
	let mut returned: Vec<Kind> = vec![];
	body.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(word) if word == "return") && !is_error(&items[1]) {
				// `return` alone returns ø: a Node, so the function's numbers are Nodes too
				let kind = if matches!(items[1].drop_meta(), Node::Empty) { Kind::Empty } else { infer_type(&items[1], &scope) };
				returned.push(kind);
				if kind.is_ref() && !returned_nodes.contains(&kind) {
					returned_nodes.push(kind);
				}
			}
		}
	});
	let last = match body.drop_meta() {
		Node::List(statements, Bracket::Curly, _) if !statements.is_empty() => &statements[statements.len() - 1],
		other => other,
	};
	// `f() := {}`, a handler `{{}}`: a body doing nothing returns ø
	let empty_block = matches!(last.drop_meta(), Node::List(items, Bracket::Curly, _) if items.is_empty());
	let last_kind = match (is_error(last), returned.first()) {
		(false, None) if empty_block => Kind::Empty,
		(true, Some(_)) if returned.contains(&Kind::Float) && returned.iter().all(|kind| !kind.is_ref()) => Kind::Float,
		(true, Some(first)) if returned.iter().all(|kind| kind == first) => *first,
		(true, Some(_)) => Kind::Empty, // returns of different kinds: a Node of unknown kind
		// `while true { …; return left }` with left a float: a float function, whatever the loop is worth
		_ => match infer_type(last, &scope) {
			Kind::Int if returned.contains(&Kind::Float) => Kind::Float,
			kind => kind,
		},
	};
	match returned_nodes.as_slice() {
		_ if last_kind.is_ref() => last_kind,
		[] => last_kind,
		[Kind::Empty] => Kind::Empty,
		[Kind::List] => Kind::List,
		_ => Kind::Empty, // Nodes of different kinds (a text here, a list there): known only at run time
	}
}

/// `number` is the exact numeric tower (Int); the other builtin type names have their own kind;
/// a list type (`list`, `list of int`, `[int]`, `int[]`, `ints`) is a List
pub(crate) fn annotated_kind(type_node: &Node) -> Option<Kind> {
	if crate::type_constructor::instance_parts_marked(type_node) {
		return Some(Kind::Key); // `p:person` of a declared type (traits::lower_conformances): an instance
	}
	let type_name = type_node.name();
	if bracketed_list_type(type_node).is_some() || names_list_type(&type_name) {
		return Some(Kind::List);
	}
	// `v:any`, as an untyped field: any value, held as a Node (std/json.wasp's to_json takes what parse_json gives)
	if type_name == crate::type_kinds::UNTYPED_FIELD {
		return Some(Kind::Empty);
	}
	type_word_kind(&type_name)
}

pub(super) fn names_list_type(type_name: &str) -> bool {
	type_name == "list" || type_name.starts_with("list of ") || plural_element_type(type_name).is_some()
}

/// The kind a builtin type word names, as annotation (`x:double`) or constructor (`double 2`)
/// The key under which Scope::function_kinds holds the declared kind of a field (as tuples::element_key holds a
/// tuple element's): `v.x` reads a float when every declared type with a field x declares it `float`
pub fn field_kind_key(field: &str) -> String {
	format!(".{field}")
}

/// The fields of the declared types whose declarations all agree on one builtin kind, keyed by field_kind_key
pub fn declared_field_kinds(registry: &crate::type_kinds::TypeRegistry) -> HashMap<String, Kind> {
	let mut kinds: HashMap<String, Option<Kind>> = HashMap::new();
	for field in registry.types().iter().flat_map(|type_def| &type_def.fields) {
		let kind = type_word_kind(&field.type_name.to_lowercase());
		kinds.entry(field_kind_key(&field.name)).and_modify(|known| if *known != kind { *known = None }).or_insert(kind);
	}
	kinds.into_iter().filter_map(|(key, kind)| Some((key, kind?))).collect()
}

/// The declared field kinds of a program
pub(super) fn program_field_kinds(program: &Node) -> HashMap<String, Kind> {
	let mut registry = crate::type_kinds::TypeRegistry::new();
	collect_all_types(&mut registry, program);
	declared_field_kinds(&registry)
}

pub fn type_word_kind(type_name: &str) -> Option<Kind> {
	match type_name {
		"number" => Some(Kind::Int),
		_ => builtin_type_kind(type_name),
	}
}

/// A parameter takes its declared `x:type` kind, else the kind of its default value (a fresh value per call); otherwise Int
pub fn param_kind(param: &Param) -> Kind {
	if let Some(kind) = param.annotation.as_ref().and_then(annotated_kind) {
		return kind;
	}
	match param.default.as_ref().map(|value| argument_literal_kind(value).unwrap_or_else(|| infer_type(value, &Scope::new()))) {
		Some(kind @ (Kind::Float | Kind::Text | Kind::List)) => kind,
		// `s="a"`: a one-character text is parsed as a Codepoint; the parameter holds any text
		Some(Kind::Codepoint) => Kind::Text,
		_ => param.used_as.unwrap_or(Kind::Int),
	}
}

/// Every definition form (`f(x) := …`, `fn`, `def`, `function`) infers its parameter and return kinds alike
pub(super) fn user_function(name: &str, params: Vec<Param>, body: &Node) -> UserFunctionDef {
	let params = with_usage_kinds(params, body);
	let return_kind = infer_function_return_kind(&params, body, &HashMap::new(), &HashMap::new(), &HashMap::new());
	UserFunctionDef { name: name.to_string(), params, body: Box::new(body.clone()), return_kind, tuple_kinds: vec![], func_index: None }
}

/// Undeclared parameters that the body indexes (`xs#2`, `it[1]`) or reads as a map (`g.keys()`, `xs.has(x)`) take a list;
/// those handed to `cell_get(c)` and the other cell words are cells
pub(super) fn with_usage_kinds(params: Vec<Param>, body: &Node) -> Vec<Param> {
	let mut indexed: HashSet<String> = HashSet::new();
	let mut called: HashSet<String> = HashSet::new();
	let mut celled: HashSet<String> = HashSet::new();
	let mut aliases: Vec<(String, String)> = vec![];
	body.visit(&mut |node| {
		if let Some(name) = crate::closures::called_closure(node) {
			called.insert(name.to_string());
		}
		if let Node::List(items, Bracket::Round, _) = node {
			if let [word, map, ..] = items.as_slice() {
				// `t.left = 3` lowers to `t = field_with(t, "left", 3)`: t is an object too
				let is_map_word = matches!(word.drop_meta(), Node::Symbol(call) if crate::library_words::MAP_WORD_FUNCTIONS.contains(&call.as_str()) || call == crate::library_words::FIELD_WITH);
				if let (true, Node::Symbol(name)) = (is_map_word, map.drop_meta()) {
					indexed.insert(name.clone());
				}
				if let (true, Node::Symbol(name)) = (crate::wasm_emitter::cells::is_cell_word(word), map.drop_meta()) {
					celled.insert(name.clone());
				}
			}
		}
		if let Some(Node::Symbol(name)) = used_sequence(node).map(Node::drop_meta) {
			indexed.insert(name.clone());
		}
		if let Node::Key(alias, Op::Assign | Op::Define, source) = node {
			if let (Node::Symbol(alias), Node::Symbol(source)) = (alias.drop_meta(), source.drop_meta()) {
				aliases.push((alias.clone(), source.clone()));
			}
		}
	});
	// `for x in xs` walks the copy `x·items = xs`: the source of a used sequence is one too
	while let Some((_, source)) = aliases.iter().find(|(alias, source)| indexed.contains(alias) && !indexed.contains(source)) {
		indexed.insert(source.clone());
	}
	params.into_iter().map(|param| {
		let used_as = if called.contains(&param.name) {
			Some(Kind::Function)
		} else if celled.contains(&param.name) {
			Some(Kind::Data)
		} else if indexed.contains(&param.name) {
			Some(Kind::List)
		} else {
			None
		};
		Param { used_as, ..param }
	}).collect()
}

/// The sequence a node uses: indexed `xs#2`, counted `#xs`, `count xs`, `xs.length`
pub(super) fn used_sequence(node: &Node) -> Option<&Node> {
	let is_counting_word = |word: &Node| matches!(word.drop_meta(), Node::Symbol(name) if is_counting_property(name) && !TYPE_WORDS_AMONG_COUNTING.contains(&name.as_str()));
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
				let Some(Node::Symbol(callee)) = items.first().map(Node::drop_meta) else { return };
				// a closure call takes Nodes (as in infer_parameters_from_calls): its helper's parameters name no kind
				if crate::closures::closure_call_arity(callee).is_some() {
					return;
				}
				let Some(called) = ctx.user_functions.get(callee) else { return };
				for (argument, called_param) in items[1..].iter().zip(&called.params) {
					let Node::Symbol(name) = argument.drop_meta() else { continue };
					let Some(index) = function.params.iter().position(|param| &param.name == name) else { continue };
					let (param, kind) = (&function.params[index], param_kind(called_param));
					if kind != Kind::Int && param_kind(param) == Kind::Int && param.annotation.is_none() && param.default.is_none() {
						forwarded.push((function.name.clone(), index, kind));
					}
				}
			});
		}
		if forwarded.is_empty() {
			return;
		}
		for (name, index, kind) in forwarded {
			ctx.user_functions.get_mut(&name).expect("collected from user functions").params[index].used_as = Some(kind);
		}
	}
}

/// The functions, imports and closure facts of a program; the same tree is analysed once (analysis_memo.rs, P91).
/// Recognizes patterns:
/// - `name(param) = body` → Key(List[name, param], Assign, body)
/// - `name := body` → Key(Symbol(name), Define, body) (uses implicit `it`)
pub fn extract_user_functions(ctx: &mut Context, node: &Node) {
	crate::analysis_memo::analysed(ctx, node, analyse_user_functions);
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
	let globals = with_closure_captures(ctx, node, globals);
	refine_return_kinds(ctx, &globals);
	// a widened parameter can make the arguments it passes on floats too: until nothing changes (each round widens one)
	let parameter_count: usize = ctx.user_functions.values().map(|function| function.params.len()).sum();
	for _ in 0..parameter_count {
		if !widen_parameters(ctx, node, &globals) {
			break;
		}
		refine_return_kinds(ctx, &globals);
	}
	crate::closures::type_closure_calls(ctx);
}

/// The kinds the user functions are known to give: each one's return kind and each value of a tuple function
/// (`(h, o) = f()` reads o's kind as `f#1`)
fn known_function_kinds(ctx: &Context) -> HashMap<String, Kind> {
	ctx.user_functions.iter().flat_map(|(name, function)| {
		let values = function.tuple_kinds.iter().enumerate().map(|(index, kind)| (crate::tuples::element_key(name, index), *kind));
		std::iter::once((name.clone(), function.return_kind)).chain(values)
	}).collect()
}

/// A parameter that the calls pass one kind other than Int takes that kind: `mul(v, 1.0 / length(v))` with length
/// returning a float, `print_tree(tree.left, prefix + "│ ")` passing a text. infer_parameters_from_calls knows only
/// literal arguments; this pass runs once the return kinds are known. It only changes undeclared Int parameters (and
/// those guessed a list that get only texts), and leaves a parameter alone when calls disagree. True when a parameter changed.
pub(super) fn widen_parameters(ctx: &mut Context, program: &Node, globals: &HashMap<String, Local>) -> bool {
	let mut function_kinds = known_function_kinds(ctx);
	function_kinds.extend(ctx.field_kinds.clone());
	let mut passed: HashMap<(String, usize), HashSet<Kind>> = HashMap::new();
	let mut main = Scope::with_function_kinds(function_kinds.clone()).with_closure_targets(ctx.closure_variable_targets.clone());
	collect_variables(program, &mut main);
	collect_argument_kinds(program, &main, ctx, &mut passed);
	for function in ctx.user_functions.values() {
		let scope = function_body_scope(&function.params, &function.body, &function_kinds, globals, &ctx.closure_variable_targets);
		collect_argument_kinds(&function.body, &scope, ctx, &mut passed);
	}
	let mut changed = false;
	for ((name, index), kinds) in passed {
		let param = &mut ctx.user_functions.get_mut(&name).expect("collected from known functions").params[index];
		// passed only values held as Nodes (a loop variable over a list parameter): a Node, no int, and no list guessed
		// from indexing (capitalize(w) for the elements of a function's list result)
		let unknown = matches!(param.used_as, None | Some(Kind::List));
		if kinds.len() == 1 && kinds.contains(&Kind::Empty) && unknown && param.annotation.is_none() && param.default.is_none() {
			param.used_as = Some(Kind::Empty);
			changed = true;
			continue;
		}
		let kinds: Vec<Kind> = kinds.into_iter().filter(|kind| *kind != Kind::Int && *kind != Kind::Empty).collect();
		let [kind] = kinds.as_slice() else { continue };
		// a parameter the body counts or indexes is guessed a list, until the calls pass it only texts
		let guessed = matches!(param.used_as, None | Some(Kind::Int)) || (param.used_as == Some(Kind::List) && *kind == Kind::Text);
		if param.annotation.is_none() && param.default.is_none() && guessed {
			param.used_as = Some(*kind);
			changed = true;
		}
	}
	changed
}

/// The user functions a list calls and their arguments: `f(a, b)`, and a task start that runs f in another instance with
/// them: `task·go(f, a, b)` (lower_tasks), `task_spawn("f", a, b)`, and `task_spawn_values("f·node", [a, b])`, which
/// calls the wrapper f·node with the list and through it f with the items (resolve_tasks)
pub(super) fn called_functions(items: &[Node]) -> Vec<(String, Vec<&Node>)> {
	let Some(Node::Symbol(head)) = items.first().map(Node::drop_meta) else { return vec![] };
	let started = match items.get(1).map(Node::drop_meta) {
		Some(Node::Text(name) | Node::Symbol(name)) => Some(name.clone()),
		_ => None,
	};
	match head.as_str() {
		crate::declarations::TASK_GO | crate::host::TASK_SPAWN => started.map(|name| (name, items[2..].iter().collect())).into_iter().collect(),
		crate::host::TASK_SPAWN_VALUES | crate::host::GUARDED_CALL => match (started, items.get(2)) {
			(Some(wrapper), Some(list)) => {
				let mut calls = vec![(wrapper.clone(), vec![list])];
				if let (Some(function), Node::List(arguments, _, _)) = (wrapper.strip_suffix(crate::declarations::NODE_WRAPPER_SUFFIX), list.drop_meta()) {
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
pub(super) fn collect_argument_kinds(node: &Node, scope: &Scope, ctx: &Context, passed: &mut HashMap<(String, usize), HashSet<Kind>>) {
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
pub(super) fn with_closure_captures(ctx: &Context, program: &Node, mut globals: HashMap<String, Local>) -> HashMap<String, Local> {
	// `xs = [w(), w()]` holds what w returns, as the emitter's capture globals do (card float-calls)
	let mut outer = Scope::with_function_kinds(known_function_kinds(ctx));
	collect_variables(program, &mut outer);
	let mut functions: Vec<&UserFunctionDef> = ctx.user_functions.values().collect();
	functions.sort_by(|a, b| a.name.cmp(&b.name));
	for function in functions {
		let enclosing = ctx.enclosing_functions.get(&function.name).and_then(|name| ctx.user_functions.get(name));
		let captured: Vec<(String, Kind)> = match enclosing {
			Some(enclosing) => {
				let mut enclosing_scope = Scope::new();
				for param in &enclosing.params {
					enclosing_scope.define_param(param.name.clone(), param_kind(param));
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
	let mut scope = Scope::new();
	collect_variables(program, &mut scope);
	scope.globals
}

/// `f -x` with a user or built-in function `f` is the call `f(-x)`: a function is never an operand of a subtraction.
/// A built-in name that the program also binds as a variable or parameter (`exp-1`) stays a subtraction.
pub fn lower_negated_calls(node: Node) -> Node {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, &node);
	let mut bound: HashSet<String> = ctx.user_functions.values().flat_map(|function| function.params.iter().map(|param| param.name.clone())).collect();
	crate::library_words::collect_assigned_names(&node, &mut bound);
	negate_calls(node, &ctx.user_functions, &bound)
}

/// The functions a word can apply: the program's functions that take arguments and the built-in real functions
pub fn applicable_function_names(node: &Node) -> HashSet<String> {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, node);
	let user_functions = ctx.user_functions.into_iter().filter(|(_, function)| !function.params.is_empty()).map(|(name, _)| name);
	user_functions.chain(crate::real::FUNCTIONS.iter().map(|name| name.to_string())).collect()
}

pub(super) fn negate_calls(node: Node, functions: &std::collections::BTreeMap<String, UserFunctionDef>, bound: &HashSet<String>) -> Node {
	let is_function = |operand: &Node| match operand.drop_meta() {
		Node::Symbol(name) => functions.get(name).is_some_and(|function| !function.params.is_empty())
			|| (crate::real::FUNCTIONS.contains(&name.as_str()) && !bound.contains(name)),
		_ => false,
	};
	match node {
		Node::Key(function, Op::Sub, argument) if is_function(&function) => {
			let negated = Node::Key(Box::new(Node::Empty), Op::Neg, Box::new(negate_calls(*argument, functions, bound)));
			Node::List(vec![*function, negated], Bracket::Round, Separator::None)
		}
		Node::Key(left, op, right) => Node::Key(Box::new(negate_calls(*left, functions, bound)), op, Box::new(negate_calls(*right, functions, bound))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| negate_calls(item, functions, bound)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(negate_calls(*node, functions, bound)), data },
		other => other,
	}
}

/// A lifted lambda is never called by name (closures.rs): its captured values take the kinds of the variables captured (a
/// parameter of the enclosing function, a literal variable), its parameters the kinds of the literal arguments of the closure
/// calls of its arity
pub(super) fn infer_closure_parameters(ctx: &mut Context, program: &Node) {
	let variable_kinds = variable_kinds(program, ctx, true);
	loop {
		let mut inferred: Vec<(String, usize, Kind)> = vec![];
		let scopes = std::iter::once((program, HashMap::new())).chain(ctx.user_functions.values().map(|function| {
			let params: HashMap<String, Kind> = function.params.iter().map(|param| (param.name.clone(), param_kind(param))).collect();
			(function.body.as_ref(), params)
		}));
		for (body, params) in scopes {
			body.visit(&mut |node| {
				let value_kind = |value: &Node| match value.drop_meta() {
					Node::Symbol(name) => params.get(name).or(variable_kinds.get(name)).copied(),
					// a cell's value is a Node of any kind (a signal's new value, lowering/signal_values.rs)
					Node::List(items, _, _) if items.first().is_some_and(|word| word.drop_meta().name() == crate::wasm_emitter::cells::CELL_GET) => Some(Kind::Empty),
					_ => argument_literal_kind(value),
				};
				if let Some((target, captured)) = crate::closures::as_closure_new(node) {
					inferred.extend(captured.iter().enumerate().filter_map(|(index, value)| Some((target.to_string(), index, value_kind(value)?))));
				}
				let Node::List(items, _, _) = node else { return };
				let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) else { return };
				let Some(arity) = crate::closures::closure_call_arity(name) else { return };
				for (target, captured) in ctx.closure_targets.iter().filter(|(target, captured)| ctx.user_functions.get(target).is_some_and(|function| function.params.len() == captured + arity)) {
					inferred.extend(items[2..].iter().enumerate().filter_map(|(index, argument)| Some((target.clone(), captured + index, value_kind(argument)?))));
				}
			});
		}
		let mut changed = false;
		for (target, index, kind) in inferred {
			let Some(param) = ctx.user_functions.get_mut(&target).and_then(|function| function.params.get_mut(index)) else { continue };
			if kind != Kind::Int && param.annotation.is_none() && param.default.is_none() && param.used_as.is_none() {
				param.used_as = Some(kind);
				changed = true;
			}
		}
		if !changed {
			return;
		}
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

/// The kind a call argument certainly has, judged from the literal alone
pub fn argument_literal_kind(argument: &Node) -> Option<Kind> {
	match argument.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::List(_, Bracket::Square, _) => Some(infer_type(argument, &Scope::new())),
		Node::Empty => Some(Kind::List),
		// `"" + n`, `"#" + x + y`: a concatenation with a text is a text
		Node::Key(_, Op::Add, _) if super::declaration_lowering::is_text(argument) => Some(Kind::Text),
		// `1.5 as float`, `x as float`: the kind the conversion names
		Node::Key(_, Op::As, target) if builtin_type_kind(&target.name()).is_some_and(|kind| kind.is_float()) => Some(Kind::Float),
		_ if crate::closures::as_closure_new(argument).is_some() => Some(Kind::Function),
		Node::List(entries, Bracket::Curly, _) if entries.iter().all(|entry| matches!(entry.drop_meta(), Node::Key(_, Op::Colon, _))) => Some(Kind::List), // a map
		_ => None,
	}
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
	program.visit(&mut |node| {
		let Node::Key(target, Op::Assign | Op::Define, value) = node else { return };
		let Node::Symbol(name) = target.drop_meta() else { return };
		let kind = argument_literal_kind(value);
		if kind.is_none() && unknown_keeps_kind {
			return;
		}
		literal.insert(name.clone());
		kinds.entry(name.clone()).and_modify(|known| if *known != kind { *known = None }).or_insert(kind);
	});
	kinds.into_iter().filter(|(name, _)| !unknown_keeps_kind || literal.contains(name)).filter_map(|(name, kind)| Some((name, kind?))).collect()
}

/// A decimal literal, `2.5` or `2.0` (an exact decimal, so its kind may be Int): a float an int parameter refuses (P49, P49b)
pub(super) fn is_decimal_literal(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Number(crate::extensions::numbers::Number::Float(_)))
}

/// Variables assigned a decimal literal (`y = 2.5`, `y = 3.0`)
pub(super) fn decimal_variables(program: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = node {
			if let (Node::Symbol(name), true) = (target.drop_meta(), is_decimal_literal(value)) {
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
	let variable_kinds = literal_variable_kinds(program, ctx);
	let argument_kind = |argument: &Node| argument_literal_kind(argument).or_else(|| match argument.drop_meta() {
		Node::Symbol(name) => variable_kinds.get(name).copied(),
		_ => None,
	});
	let mut argument_kinds: HashMap<(String, usize), Vec<Kind>> = HashMap::new();
	// a float passed to a declared int parameter loses digits: a compile error (P49)
	let mut float_for_int: Vec<String> = Vec::new();
	// a parameter of the same name shadows the variable (as in literal_variable_kinds)
	let mut decimal_variables = decimal_variables(program);
	ctx.user_functions.values().flat_map(|function| &function.params).for_each(|param| { decimal_variables.remove(&param.name); });
	let has_digits_an_int_loses = |argument: &Node| is_decimal_literal(argument) || argument_kind(argument) == Some(Kind::Float)
		|| matches!(argument.drop_meta(), Node::Symbol(name) if decimal_variables.contains(name));
	program.visit(&mut |node| {
		let Node::List(items, _, _) = node else { return };
		for (name, arguments) in called_functions(items) {
			// a closure call takes Nodes, or the numbers type_closure_calls gives both it and its entries
			if crate::closures::closure_call_arity(&name).is_some() {
				continue;
			}
			let Some(function) = ctx.user_functions.get(&name) else { continue };
			for (index, argument) in arguments.into_iter().enumerate().take(function.params.len()) {
				// `real`, `exact` are Ints that may hold a ratio: only a whole type loses a decimal's digits
				let declared_int = function.params[index].annotation.as_ref().is_some_and(|annotation| crate::analyzer::checks::is_whole_type(&annotation.name()));
				if declared_int && has_digits_an_int_loses(argument) {
					// a whole float serializes as `2`; its decimal point is what makes it no int
					let written = match argument.drop_meta() {
						Node::Number(crate::extensions::numbers::Number::Float(value)) if value.fract() == 0.0 => format!("{value:.1}"),
						other => other.serialize(),
					};
					float_for_int.push(format!("{written} is no int: write {written} as int"));
				}
				if let Some(kind) = argument_kind(argument) {
					let kinds = argument_kinds.entry((name.clone(), index)).or_default();
					if !kinds.contains(&kind) {
						kinds.push(kind);
					}
				}
			}
		}
	});
	ctx.parameter_conflicts.extend(float_for_int);
	for ((name, index), kinds) in argument_kinds {
		let function = ctx.user_functions.get_mut(&name).expect("call sites were collected from known functions");
		let param = &mut function.params[index];
		if param.annotation.is_some() || param.default.is_some() {
			continue;
		}
		match kinds.as_slice() {
			[Kind::Int] => {}
			[kind] => param.used_as = Some(*kind),
			[first, second, ..] => ctx.parameter_conflicts.push(format!(
				"{name} is called with {} and {} for parameter {}: annotate it, e.g. {}:any",
				kind_with_article(*first), kind_with_article(*second), param.name, param.name)),
			[] => {}
		}
	}
}

/// Infer every return kind again knowing all user functions (they shadow FFI names, recursion assumes Int first),
/// until the kinds settle: a call of a float-returning function is itself Float
pub(super) fn refine_return_kinds(ctx: &mut Context, globals: &HashMap<String, Local>) {
	let mut function_kinds: HashMap<String, Kind> = ctx.user_functions.keys().map(|name| (name.clone(), Kind::Int)).collect();
	function_kinds.extend(ctx.field_kinds.clone());
	for _ in 0..=ctx.user_functions.len() {
		let inferred: Vec<(String, Kind)> = ctx.user_functions.values()
			.map(|function| {
				let kind = crate::closures::closure_call_kind(&function.name, ctx, &function_kinds)
					.unwrap_or_else(|| infer_function_return_kind(&function.params, &function.body, &function_kinds, globals, &ctx.closure_variable_targets));
				// `if t == ø { return [] }; return f(t.left) + [x]`: assumed to return an Int, the recursion makes an
				// error of the last value; assumed to return a Node of unknown kind, it settles
				let kind = if kind == Kind::Error && function_kinds.get(&function.name) == Some(&Kind::Int) {
					let mut unknown_recursion = function_kinds.clone();
					unknown_recursion.insert(function.name.clone(), Kind::Empty);
					infer_function_return_kind(&function.params, &function.body, &unknown_recursion, globals, &ctx.closure_variable_targets)
				} else {
					kind
				};
				(function.name.clone(), kind)
			})
			.collect();
		let tuples: Vec<(String, Kind)> = ctx.user_functions.values()
			.flat_map(|function| {
				let kinds = infer_tuple_kinds(&function.params, &function.body, &function_kinds, globals, &ctx.closure_variable_targets);
				kinds.into_iter().enumerate().map(|(index, kind)| (crate::tuples::element_key(&function.name, index), kind)).collect::<Vec<_>>()
			})
			.collect();
		let settled = inferred.iter().chain(&tuples).all(|(name, kind)| function_kinds.get(name) == Some(kind));
		function_kinds.extend(inferred);
		function_kinds.extend(tuples);
		if settled {
			break;
		}
	}
	for function in ctx.user_functions.values_mut() {
		function.return_kind = function_kinds[&function.name];
		function.tuple_kinds = (0..crate::tuples::tuple_arity(&function.body).unwrap_or(0))
			.map(|index| function_kinds[&crate::tuples::element_key(&function.name, index)])
			.collect();
	}
}

pub(super) fn extract_user_functions_inner(ctx: &mut Context, node: &Node) {
	extract_user_functions_in(ctx, node, None);
}

/// Nested `def` / `f() = …` inside a function body becomes a top-level UserFunctionDef named `outer·inner`, with call
/// sites in the enclosing body rewritten to that mangled name (wiki/charged.md §3 nested defs; card g-qUkY step 1).
pub(super) fn extract_user_functions_in(ctx: &mut Context, node: &Node, enclosing: Option<&str>) {
	let node = node.drop_meta();
	match node {
		// Pattern: name(param1, param2, ...) = body
		Node::Key(left, Op::Assign, body) => {
			if let Node::List(items, bracket, _) = left.drop_meta() {
				if !items.is_empty() {
					if let Node::Symbol(name) = items[0].drop_meta() {
						register_user_function(ctx, name, extract_params(items, bracket), body, enclosing);
						return;
					}
				}
			}
			extract_user_functions_in(ctx, left, enclosing);
			extract_user_functions_in(ctx, body, enclosing);
		}
		// Pattern: name x := body (with explicit parameter x using $0 or `it`)
		Node::Key(left, Op::Define, body) => {
			if let Node::List(items, bracket, _) = left.drop_meta() {
				if !items.is_empty() {
					if let Node::Symbol(name) = items[0].drop_meta() {
						let params = extract_params(items, bracket);
						let implicit_param = uses_dollar_param(body) || uses_it(body);
						let written_call = *bracket == Bracket::Round; // `f() := [1, 2]` defines f without parameters
						if !params.is_empty() || implicit_param || written_call {
							let params = if params.is_empty() && implicit_param { vec![Param::untyped("it")] } else { params };
							register_user_function(ctx, name, params, body, enclosing);
							return;
						}
					}
				}
			}
			// Pattern: name := body (implicit `it` parameter, or none for a statement block)
			if let Node::Symbol(name) = left.drop_meta() {
				if uses_it(body) || uses_dollar_param(body) || is_statement_block(body) {
					// a block of statements takes `it` when it reads it outside its own `for 1..n {…it…}` loops
					let takes_it = !is_statement_block(body) || uses_it_outside_loops(body) || uses_dollar_param(body);
					let params = if takes_it { vec![Param::untyped("it")] } else { vec![] };
					register_user_function(ctx, name, params, body, enclosing);
					return;
				}
			}
			extract_user_functions_in(ctx, left, enclosing);
			extract_user_functions_in(ctx, body, enclosing);
		}
		// Check for def/fun/fn syntax
		Node::List(items, _, _) => {
			if items.len() >= 2 {
				if let Node::Symbol(s) = items[0].drop_meta() {
					if is_function_keyword(s) {
						if let Some(func_def) = extract_def_function(&items[1..]) {
							register_user_function(ctx, &func_def.name, func_def.params, &func_def.body, enclosing);
							return;
						}
					}
				}
			}
			for item in items {
				extract_user_functions_in(ctx, item, enclosing);
			}
		}
		Node::Key(left, _, right) => {
			extract_user_functions_in(ctx, left, enclosing);
			extract_user_functions_in(ctx, right, enclosing);
		}
		_ => {}
	}
}

/// Between an enclosing function and a nested `def` lifted out of its body: `outer·inner`
pub(super) const NESTED_DEF_SEPARATOR: &str = "·";

pub(crate) fn qualify_nested_name(enclosing: Option<&str>, name: &str) -> String {
	match enclosing {
		Some(parent) => format!("{parent}{NESTED_DEF_SEPARATOR}{name}"),
		None => name.to_string(),
	}
}

pub(super) fn register_user_function(ctx: &mut Context, short_name: &str, params: Vec<Param>, body: &Node, enclosing: Option<&str>) {
	let name = qualify_nested_name(enclosing, short_name);
	let (body, renames) = lift_nested_defs_from_body(ctx, body.clone(), &name);
	let body = rename_nested_calls(body, &renames);
	// a sibling calls a nested function too: `def sibling(){ inner() }` beside `def inner()` in outer
	let nested_prefix = format!("{name}{NESTED_DEF_SEPARATOR}");
	for function in ctx.user_functions.values_mut().filter(|function| function.name.starts_with(&nested_prefix)) {
		*function.body = rename_nested_calls(function.body.as_ref().clone(), &renames);
	}
	if let Some(parent) = enclosing {
		ctx.enclosing_functions.insert(name.clone(), parent.to_string());
	}
	ctx.user_functions.insert(name.clone(), user_function(&name, params, &body));
}

/// Nested function definitions in `body` become UserFunctionDefs named `parent·short`; their statements stay (emit skips
/// them) and call sites are renamed via the returned map.
pub(super) fn lift_nested_defs_from_body(ctx: &mut Context, body: Node, parent: &str) -> (Node, HashMap<String, String>) {
	let mut renames = HashMap::new();
	let body = lift_nested_defs_walk(ctx, body, parent, &mut renames);
	(body, renames)
}

pub(super) fn lift_nested_defs_walk(ctx: &mut Context, node: Node, parent: &str, renames: &mut HashMap<String, String>) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(lift_nested_defs_walk(ctx, *node, parent, renames)), data },
		Node::Key(left, op @ (Op::Assign | Op::Define), body) => {
			if let Node::List(items, bracket, separator) = left.drop_meta() {
				if let Some(Node::Symbol(short)) = items.first().map(Node::drop_meta) {
					let mangled = qualify_nested_name(Some(parent), short);
					renames.insert(short.clone(), mangled.clone());
					register_user_function(ctx, short, extract_params(items, bracket), &body, Some(parent));
					// LHS must use the mangled name so emit's defined_function_name finds it in user_functions
					let mut items = items.clone();
					items[0] = Node::Symbol(mangled);
					return Node::Key(Box::new(Node::List(items, bracket.clone(), separator.clone())), op, body);
				}
			}
			Node::Key(
				Box::new(lift_nested_defs_walk(ctx, *left, parent, renames)),
				op,
				Box::new(lift_nested_defs_walk(ctx, *body, parent, renames)),
			)
		}
		Node::Key(left, op, right) => Node::Key(
			Box::new(lift_nested_defs_walk(ctx, *left, parent, renames)),
			op,
			Box::new(lift_nested_defs_walk(ctx, *right, parent, renames)),
		),
		// quoted data (the program a run-time block carries) is not code of this body
		quoted if crate::run_time_blocks::is_data(&quoted) => quoted,
		Node::List(items, bracket, separator) => {
			if items.len() >= 2 {
				if let Node::Symbol(keyword) = items[0].drop_meta() {
					if is_function_keyword(keyword) {
						if let Some(func_def) = extract_def_function(&items[1..]) {
							renames.insert(func_def.name.clone(), qualify_nested_name(Some(parent), &func_def.name));
							register_user_function(ctx, &func_def.name, func_def.params, &func_def.body, Some(parent));
							return Node::List(items, bracket, separator);
						}
					}
				}
			}
			Node::List(
				items.into_iter().map(|item| lift_nested_defs_walk(ctx, item, parent, renames)).collect(),
				bracket,
				separator,
			)
		}
		other => other,
	}
}

/// `inner()` / `inner()+1` / `apply(inner)` inside the enclosing body: the name becomes `outer·inner`
pub(super) fn rename_nested_calls(node: Node, renames: &HashMap<String, String>) -> Node {
	if renames.is_empty() {
		return node;
	}
	match node {
		Node::Symbol(name) if renames.contains_key(&name) => Node::Symbol(renames[&name].clone()),
		Node::Meta { node, data } => Node::Meta { node: Box::new(rename_nested_calls(*node, renames)), data },
		Node::Key(left, op, right) => Node::Key(
			Box::new(rename_nested_calls(*left, renames)),
			op,
			Box::new(rename_nested_calls(*right, renames)),
		),
		Node::List(items, bracket, separator) => {
			let mut items: Vec<Node> = items.into_iter().map(|item| rename_nested_calls(item, renames)).collect();
			if bracket == Bracket::Round {
				if let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) {
					if let Some(mangled) = renames.get(name) {
						items[0] = Node::Symbol(mangled.clone());
					}
				}
			}
			Node::List(items, bracket, separator)
		}
		other => other,
	}
}

/// The parameters after the function name in a signature list. Only a spaced signature `f int x` pairs a type word with
/// the next name; in the call form `f(T, y)` every item is a parameter (the parser made `f(int x)` the item `x:int`)
pub(super) fn extract_params(signature: &[Node], bracket: &Bracket) -> Vec<Param> {
	// `f(xs: list of int, y)`: the comma groups `xs:list of int` as one spaced item
	let flat: Vec<&Node> = signature.iter().skip(1).flat_map(|item| match item.drop_meta() {
		Node::List(parts, _, Separator::Space | Separator::None) if matches!(parts.first().map(Node::drop_meta), Some(Node::Key(_, Op::Colon, _))) => parts.iter().collect(),
		_ => vec![item],
	}).collect();
	let mut items = flat.into_iter().peekable();
	let mut params = vec![];
	while let Some(item) = items.next() {
		let type_first_name = match (item.drop_meta(), items.peek().map(|next| next.drop_meta())) {
			(Node::Symbol(type_name), Some(Node::Symbol(name))) if *bracket != Bracket::Round && type_word_kind(type_name).is_some() => Some(name),
			_ => None,
		};
		match type_first_name {
			Some(name) => {
				params.push(Param { name: name.clone(), annotation: Some(item.clone()), default: None, used_as: None });
				items.next();
			}
			None => params.extend(extract_param(item).map(|param| with_list_annotation(param, &mut items))),
		}
	}
	params
}

/// `xs: T list` and `xs: list of T` (the parser leaves `list`, or `of T`, as the next items) annotate xs as a `list of T`:
/// T a type word, a declared type or a trait (`sort(xs: Comparable list)`)
pub(super) fn with_list_annotation<'a>(param: Param, items: &mut std::iter::Peekable<impl Iterator<Item = &'a Node>>) -> Param {
	let Some(annotation) = param.annotation.as_ref().map(Node::name) else { return param };
	let next_word = |items: &mut std::iter::Peekable<_>| match items.peek().map(|next: &&Node| next.drop_meta()) {
		Some(Node::Symbol(word)) => Some(word.clone()),
		_ => None,
	};
	let element = match next_word(items).as_deref() {
		Some(LIST_WORD) => annotation,
		Some(OF_WORD) if annotation == LIST_WORD => {
			items.next();
			match next_word(items) {
				Some(element) => element,
				None => return param,
			}
		}
		_ => return param,
	};
	items.next();
	Param { annotation: Some(Node::Symbol(format!("{LIST_OF_PREFIX}{element}"))), ..param }
}

/// Extract parameter name, annotated kind and optional default value from a parameter node
pub(super) fn extract_param(item: &Node) -> Option<Param> {
	match item.drop_meta() {
		Node::Symbol(s) => Some(Param::untyped(s)),
		Node::Key(n, Op::Colon, type_name) => {
			if let Node::Symbol(s) = n.drop_meta() {
				Some(Param { name: s.clone(), annotation: Some(type_name.as_ref().clone()), default: None, used_as: None })
			} else {
				None
			}
		}
		// `b=2` and the typed `b:int=2`
		Node::Key(declared, Op::Assign, default) => {
			let param = extract_param(declared)?;
			Some(Param { default: Some(default.as_ref().clone()), ..param })
		}
		_ => None,
	}
}

/// Check if a node uses the implicit `it` parameter
pub(super) fn uses_it(node: &Node) -> bool {
	let node = node.drop_meta();
	match node {
		Node::Symbol(s) if s == "it" => true,
		Node::Key(left, _, right) => uses_it(left) || uses_it(right),
		Node::List(items, _, _) => items.iter().any(uses_it),
		_ => false,
	}
}

/// A body of `name := body` that reads `it` or `$0`: a function of that parameter
pub(crate) fn takes_implicit_parameter(body: &Node) -> bool {
	uses_it(body) || uses_dollar_param(body)
}

/// Check if a node uses $n parameter references (e.g., $0, $1)
pub(super) fn uses_dollar_param(node: &Node) -> bool {
	let node = node.drop_meta();
	match node {
		Node::Symbol(s) if s.starts_with('$') && s[1..].parse::<u32>().is_ok() => true,
		Node::Key(left, _, right) => uses_dollar_param(left) || uses_dollar_param(right),
		Node::List(items, _, _) => items.iter().any(uses_dollar_param),
		_ => false,
	}
}

/// Extract function from def/fun/fn syntax
pub(crate) fn extract_def_function(items: &[Node]) -> Option<UserFunctionDef> {
	if items.is_empty() {
		return None;
	}
	let first = items[0].drop_meta();

	// Pattern 1: def (name params...): body
	if let Node::Key(sig, Op::Colon, body) = first {
		if let Node::List(sig_items, bracket, _) = sig.drop_meta() {
			if !sig_items.is_empty() {
				if let Node::Symbol(name) = sig_items[0].drop_meta() {
					return Some(user_function(name, extract_params(sig_items, bracket), body));
				}
			}
		}
	}

	// Pattern 2: def ((name params...) {body})
	if let Node::List(inner_items, _, _) = first {
		if inner_items.len() >= 2 {
			if let Node::List(sig_items, _, _) = inner_items[0].drop_meta() {
				if !sig_items.is_empty() {
					if let Node::Symbol(name) = sig_items[0].drop_meta() {
						let params: Vec<Param> = sig_items
							.iter()
							.skip(1)
							.flat_map(|item| {
								match item.drop_meta() {
									Node::List(param_items, _, _) => {
										param_items.iter().filter_map(extract_param).collect::<Vec<_>>()
									}
									_ => extract_param(item).into_iter().collect(),
								}
							})
							.collect();
						return Some(user_function(name, params, &inner_items[1]));
					}
				}
			}
		}
	}
	None
}
