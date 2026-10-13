//! User functions: extraction, parameter and return kinds from bodies and calls, closures, negated calls

use super::*;
use crate::lowering::nodes::children_rewritten;

mod definitions;
pub(crate) use definitions::*;

/// The type word of a value that is an int or a float
const NUMBER_TYPE: &str = "number";
/// The printing words, which `print #xs` and `print -1` call like a function of the program (card print-people)
const PRINT_WORDS: [&str; 2] = ["print", "puts"];

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
pub(crate) fn computed_literal_kind(value: &Node) -> Option<Kind> {
	match value.drop_meta() {
		Node::Key(left, op, right) if op.is_arithmetic() && !matches!(left.drop_meta(), Node::Empty) => {
			// exact: whole or not by its value, `7/2*2` is 7, `33/10*2` is 33/5
			if let Some(exact) = exact_literal_value(value) {
				return Some(if exact.is_integer() { Kind::Int } else { Kind::Float });
			}
			match (operand_kind(left)?, operand_kind(right)?) {
				(Kind::Int, Kind::Int) => Some(Kind::Int),
				(Kind::Float | Kind::Int, Kind::Float | Kind::Int) => Some(Kind::Float),
				// `"5"*3` repeats the text (card repeat-int)
				(left_kind, right_kind) if super::inference::repeats_text(left_kind, op, right_kind) => {
					Some(Kind::Text)
				}
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
		Node::Number(number @ (Number::Int(_) | Number::BigInt(_) | Number::Quotient(..) | Number::BigQuotient(_))) => {
			Some(number.to_rational())
		}
		Node::Key(nothing, Op::Neg | Op::Sub, operand) if matches!(nothing.drop_meta(), Node::Empty) => {
			Some(exact_literal_value(operand)?.neg())
		}
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
pub fn function_body_scope(
	params: &[Param],
	body: &Node,
	function_kinds: &HashMap<String, Kind>,
	globals: &HashMap<String, Local>,
	closure_variable_targets: &HashMap<String, HashSet<String>>,
) -> Scope {
	let mut scope =
		Scope::with_function_kinds(function_kinds.clone()).with_closure_targets(closure_variable_targets.clone());
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
	function_kinds: &HashMap<String, Kind>,
	closure_variable_targets: &HashMap<String, HashSet<String>>,
) -> bool {
	let mut widened_any = false;
	for function in functions {
		let scope = function_body_scope(
			&function.params,
			&function.body,
			function_kinds,
			globals,
			closure_variable_targets,
		);
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
pub(super) fn infer_tuple_kinds(
	params: &[Param],
	body: &Node,
	function_kinds: &HashMap<String, Kind>,
	globals: &HashMap<String, Local>,
	closure_variable_targets: &HashMap<String, HashSet<String>>,
) -> Vec<Kind> {
	let Some(arity) = crate::tuples::tuple_arity(body) else {
		return vec![];
	};
	let scope = function_body_scope(params, body, function_kinds, globals, closure_variable_targets);
	let mut kinds: Vec<Option<Kind>> = vec![None; arity];
	body.visit(&mut |node| {
		for (kind, value) in kinds
			.iter_mut()
			.zip(crate::tuples::returned_values(node).unwrap_or_default())
		{
			let value_kind = infer_type(value, &scope);
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

pub(super) fn infer_function_return_kind(
	params: &[Param],
	body: &Node,
	function_kinds: &HashMap<String, Kind>,
	globals: &HashMap<String, Local>,
	closure_variable_targets: &HashMap<String, HashSet<String>>,
) -> Kind {
	if crate::tuples::tuple_arity(body).is_some() {
		return Kind::List; // used whole, a tuple function's values are packed into a list
	}
	let scope = function_body_scope(params, body, function_kinds, globals, closure_variable_targets);
	// `return error "…"` is the failure path: it does not decide what the function returns
	let is_error = |value: &Node| {
		let value = match value.drop_meta() {
			Node::List(items, _, _) if items.len() == 2 && items[0].is_symbol("return") => &items[1],
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
			if items.len() == 2 && items[0].is_symbol("return") && !is_error(&items[1]) {
				// `return` alone returns ø: a Node, so the function's numbers are Nodes too
				let kind = if matches!(items[1].drop_meta(), Node::Empty) {
					Kind::Empty
				} else {
					infer_type(&items[1], &scope)
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
		_ => match infer_type(last, &scope) {
			// a loop the function returns from inside: the returns decide, the loop's own value (a number or ø) does not
			Kind::Data if is_loop(last) && !returned.is_empty() => {
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
		_ if last_kind.is_ref() => last_kind,
		[] => last_kind,
		[Kind::Empty] => Kind::Empty,
		[Kind::List] => Kind::List,
		_ => Kind::Empty, // Nodes of different kinds (a text here, a list there): known only at run time
	}
}

/// `while c do body`, what every loop lowers to
fn is_loop(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(condition, Op::Do, _) if matches!(condition.drop_meta(), Node::Key(_, Op::While, _)))
}

/// `number` is the exact numeric tower (Int); the other builtin type names have their own kind;
/// a list type (`list`, `list of int`, `[int]`, `int[]`, `ints`) is a List
pub(crate) fn annotated_kind(type_node: &Node) -> Option<Kind> {
	if crate::type_constructor::instance_parts_marked(type_node) {
		return Some(Kind::Key); // `p:person` of a declared type (traits::lower_conformances): an instance
	}
	if crate::uncertain::is_interval_number(type_node) {
		return Some(Kind::Empty); // `a:number` given a ± value: a Node, checked at run time (wasm_emitter admitted)
	}
	let type_name = type_node.name();
	if bracketed_list_type(type_node).is_some() || names_list_type(&type_name) {
		return Some(Kind::List);
	}
	// `v:any`, as an untyped field: any value, held as a Node (lib/json.warp's to_json takes what parse_json gives)
	// `x: int or text` (an inline union, card inline-union) and `s: Circle or Square` (card union-types) likewise
	if names_empty_type(type_node) {
		return Some(Kind::Empty);
	}
	// `m:map`, `m: map of int`: a map, held as a Node; `{a:1}` given to it is upgraded to the map of its entry (one_entry_maps.rs)
	if type_name == crate::type_kinds::UNTYPED_FIELD || is_map_type(&type_name) || union_parts(&type_name).is_some() {
		return Some(Kind::Empty);
	}
	type_word_kind(&type_name)
}

pub(crate) fn is_map_type(type_name: &str) -> bool {
	type_name == super::checks::MAP_TYPE || type_name.starts_with(super::checks::MAP_TYPE_PREFIX)
}

/// `x:ø` / `x:nil` / `x:unit`: the empty type, ø is held as a Node
pub(super) fn names_empty_type(type_node: &Node) -> bool {
	matches!(type_node.drop_meta(), Node::Empty)
		|| crate::type_tests::canonical_spec_word(&type_node.name()) == crate::type_tests::EMPTY_TYPE
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
		let type_name = field.type_name.to_lowercase();
		// a number field holds an int or a float: read at run time (card number-field)
		let kind = type_word_kind(&type_name).filter(|_| type_name != NUMBER_TYPE);
		kinds
			.entry(field_kind_key(&field.name))
			.and_modify(|known| {
				if *known != kind {
					*known = None
				}
			})
			.or_insert(kind);
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
		NUMBER_TYPE => Some(Kind::Int),
		_ => builtin_type_kind(type_name),
	}
}

/// An unannotated parameter without a default, or with an Int one (`within = 0`), takes the kind its callers pass:
/// `f(1, w = 1e-9)` gives w a float
fn takes_passed_kind(param: &Param) -> bool {
	param.annotation.is_none()
		&& param
			.default
			.as_ref()
			.is_none_or(|value| argument_literal_kind(value) == Some(Kind::Int))
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
	let return_kind = infer_function_return_kind(&params, &body, &HashMap::new(), &HashMap::new(), &HashMap::new());
	UserFunctionDef {
		name: name.to_string(),
		params,
		body: Box::new(body),
		return_kind,
		tuple_kinds: vec![],
		func_index: None,
	}
}

/// Undeclared parameters that the body indexes (`xs#2`, `it[1]`) or reads as a map (`g.keys()`, `xs.has(x)`) take a list;
/// those handed to `cell_get(c)` and the other cell words are cells
pub(super) fn with_usage_kinds(params: Vec<Param>, body: &Node) -> Vec<Param> {
	let mut indexed: HashSet<String> = HashSet::new();
	let mut called: HashSet<String> = HashSet::new();
	let mut celled: HashSet<String> = HashSet::new();
	let mut texted: HashSet<String> = HashSet::new();
	let mut foreign: HashSet<String> = HashSet::new();
	let mut tested: HashSet<String> = HashSet::new();
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
		if let Some(name) = joined_to_text(node) {
			texted.insert(name.to_string());
		}
		foreign.extend(given_to_foreign_code(node).map(str::to_string));
		tested.extend(kind_tested(node).map(str::to_string));
		// `xs = xs + [420]`, `xs += [420]` (what `xs.add(420)` lowers to): xs is a list, called or not
		if let Some(name) = joined_to_list(node) {
			indexed.insert(name.to_string());
		}
		if let Node::Key(list, Op::AddAssign, added) = node {
			if let (Node::Symbol(name), Node::List(_, Bracket::Square, _)) = (list.drop_meta(), added.drop_meta()) {
				indexed.insert(name.clone());
			}
		}
		if let Node::Key(alias, Op::Assign | Op::Define, source) = node {
			if let (Node::Symbol(alias), Node::Symbol(source)) = (alias.drop_meta(), source.drop_meta()) {
				aliases.push((alias.clone(), source.clone()));
			}
		}
	});
	// `for x in xs` walks the copy `x·items = xs`: the source of a used sequence is one too
	while let Some((_, source)) = aliases
		.iter()
		.find(|(alias, source)| indexed.contains(alias) && !indexed.contains(source))
	{
		indexed.insert(source.clone());
	}
	params
		.into_iter()
		.map(|param| {
			let used_as = if called.contains(&param.name) {
				Some(Kind::Function)
			} else if celled.contains(&param.name) || foreign.contains(&param.name) {
				Some(Kind::Data)
			} else if indexed.contains(&param.name) {
				// a counted sequence joined to a text is a text; joined alone it may be any value (`"set " + value`)
				Some(if texted.contains(&param.name) {
					Kind::Text
				} else {
					Kind::List
				})
			} else {
				None
			};
			// `if x is list then … else x`: a parameter tested for its kind takes any value, as if written `x:any` (card recursive-sum)
			let takes_any = tested.contains(&param.name) && param.annotation.is_none() && param.default.is_none();
			let annotation = if takes_any {
				Some(Node::Symbol(crate::type_kinds::UNTYPED_FIELD.to_string()))
			} else {
				param.annotation
			};
			Param {
				used_as,
				annotation,
				..param
			}
		})
		.collect()
}

/// `is_type(x, "list")`, what `x is list` lowers to: the name whose kind is tested
fn kind_tested(node: &Node) -> Option<&str> {
	let Node::List(items, _, _) = node else { return None };
	match items.as_slice() {
		[word, tested, _] if word.is_symbol(crate::type_tests::IS_TYPE) => match tested.drop_meta() {
			Node::Symbol(name) => Some(name),
			_ => None,
		},
		_ => None,
	}
}

/// The names a foreign call takes as its receiver or arguments (lowering/foreign_modules.rs): values of any kind, as
/// foreign code gives them (a warp function called back by JavaScript, card js-callbacks)
fn given_to_foreign_code(node: &Node) -> impl Iterator<Item = &str> {
	let given = match node {
		Node::List(items, Bracket::Round, _) if call_of(node, crate::host::FOREIGN_CALL) => match items.as_slice() {
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
	joined_to(node, is_text)
}

/// The name `+` joins to a list literal: `xs + [x]`, `[x] + xs`
fn joined_to_list(node: &Node) -> Option<&str> {
	joined_to(node, |node| matches!(node, Node::List(_, Bracket::Square, _)))
}

fn call_of(node: &Node, word: &str) -> bool {
	matches!(node, Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(call)) if call == word))
}

fn is_text(node: &Node) -> bool {
	matches!(node, Node::Text(_) | Node::Char(_)) || call_of(node, crate::warp_parser::TEXT_TIMES)
}

/// The name joined by `+` to a value that `is_joined` accepts
fn joined_to(node: &Node, is_joined: fn(&Node) -> bool) -> Option<&str> {
	fn joined_name(node: &Node) -> Option<&str> {
		match node {
			Node::Symbol(name) => Some(name),
			Node::List(items, _, _) if call_of(node, crate::library_words::SLICE) => {
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
	let is_counting_word = |word: &Node| matches!(word.drop_meta(), Node::Symbol(name) if is_counting_word(name));
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
			ctx.user_functions
				.get_mut(&name)
				.expect("collected from user functions")
				.params[index]
				.used_as = Some(kind);
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
	let return_kinds: HashMap<String, Kind> = ctx
		.user_functions
		.iter()
		.map(|(name, function)| (name.clone(), function.return_kind))
		.collect();
	// `global g = 0.3; g = f(-9)` with an f that returns a Node: only the return kinds tell that main's g holds one
	let widened_by_main = widen_globals(
		&mut globals,
		&declared_globals_in(Scope::with_function_kinds(return_kinds.clone()), node),
	);
	if widen_globals_by_functions(
		&mut globals,
		ctx.user_functions.values(),
		&return_kinds,
		&ctx.closure_variable_targets,
	) | widened_by_main
	{
		refine_return_kinds(ctx, &globals);
	}
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
		let param = &mut ctx
			.user_functions
			.get_mut(&name)
			.expect("collected from known functions")
			.params[index];
		// passed only values held as Nodes (a loop variable over a list parameter): a Node, no int, and no list guessed
		// from indexing (capitalize(w) for the elements of a function's list result)
		// passed a Node and else only ints (`rect(x, levels[i], …)` and `rect(x, 2, …)`): the Node may hold a float, which an
		// int parameter refused at run time ("not an int"), so it takes the Node
		let unknown = matches!(param.used_as, None | Some(Kind::List | Kind::Int));
		let only_nodes_and_ints = kinds.iter().all(|kind| matches!(kind, Kind::Empty | Kind::Int));
		if kinds.contains(&Kind::Empty) && only_nodes_and_ints && unknown && takes_passed_kind(param) {
			param.used_as = Some(Kind::Empty);
			changed = true;
			continue;
		}
		let kinds: Vec<Kind> = kinds
			.into_iter()
			.filter(|kind| *kind != Kind::Int && *kind != Kind::Empty)
			.collect();
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
/// them: `task·go(f, a, b)` (lower_tasks), `task_spawn("f", a, b)`, and `task_spawn_values("f·node", [a, b])`, which
/// calls the wrapper f·node with the list and through it f with the items (resolve_tasks)
pub(super) fn called_functions(items: &[Node]) -> Vec<(String, Vec<&Node>)> {
	let Some(Node::Symbol(head)) = items.first().map(Node::drop_meta) else {
		return vec![];
	};
	let started = match items.get(1).map(Node::drop_meta) {
		Some(Node::Text(name) | Node::Symbol(name)) => Some(name.clone()),
		_ => None,
	};
	match head.as_str() {
		crate::declarations::TASK_GO | crate::host::TASK_SPAWN => started
			.map(|name| (name, items[2..].iter().collect()))
			.into_iter()
			.collect(),
		crate::host::TASK_SPAWN_VALUES | crate::host::GUARDED_CALL => match (started, items.get(2)) {
			(Some(wrapper), Some(list)) => {
				let mut calls = vec![(wrapper.clone(), vec![list])];
				if let (Some(function), Node::List(arguments, _, _)) = (
					wrapper.strip_suffix(crate::declarations::NODE_WRAPPER_SUFFIX),
					list.drop_meta(),
				) {
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
						passed
							.entry((name.clone(), index))
							.or_default()
							.insert(infer_type(argument, scope));
					}
				}
			}
			items
				.iter()
				.for_each(|item| collect_argument_kinds(item, scope, ctx, passed));
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
		let enclosing = ctx
			.enclosing_functions
			.get(&function.name)
			.and_then(|name| ctx.user_functions.get(name));
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
			globals
				.entry(name.clone())
				.or_insert_with(|| binding.unwrap_or_else(|| Local::new(0, name, kind)));
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

/// `f -x` with a callable `f` is the call `f(-x)`, and `print #xs` is `print(#xs)` (card print-people): a user function,
/// a real function, print, a rounding word (`floor -2.5`), a type (`int -3`, `hex -1`) or a library word of one value, as the math words
/// take `abs -2`. A minus written apart, `f - 2`, subtracts, as `abs - 2` does (warp_parser::SpacedMinus). A built-in name
/// that the program also binds as a variable or parameter (`exp-1`) stays a subtraction.
pub fn lower_negated_calls(node: Node) -> Node {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, &node);
	let mut bound: HashSet<String> = ctx
		.user_functions
		.values()
		.flat_map(|function| function.params.iter().map(|param| param.name.clone()))
		.collect();
	crate::library_words::collect_assigned_names(&node, &mut bound);
	bound.extend(crate::soft_keywords::names_given(&node)); // fields and lambda parameters too: `size -1` of a field size
	negate_calls(node, &ctx.user_functions, &bound)
}

/// The functions a word can apply: the program's functions that take arguments and the built-in real functions
pub fn applicable_function_names(node: &Node) -> HashSet<String> {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, node);
	let user_functions = ctx
		.user_functions
		.into_iter()
		.filter(|(_, function)| !function.params.is_empty())
		.map(|(name, _)| name);
	user_functions
		.chain(crate::real::FUNCTIONS.iter().map(|name| name.to_string()))
		.collect()
}

pub(super) fn negate_calls(
	node: Node,
	functions: &std::collections::BTreeMap<String, UserFunctionDef>,
	bound: &HashSet<String>,
) -> Node {
	// `int[100]` is the subscript `int#101` of a typed array: a type or library word takes no count
	let is_function = |operand: &Node, op: Op| match operand.drop_meta() {
		Node::Symbol(name) if functions.contains_key(name) => {
			functions.get(name).is_some_and(|function| !function.params.is_empty())
		}
		Node::Symbol(name) if op == Op::Hash => {
			(crate::real::FUNCTIONS.contains(&name.as_str()) || PRINT_WORDS.contains(&name.as_str()))
				&& !bound.contains(name)
		}
		Node::Symbol(name) => is_builtin_callable(name) && !bound.contains(name),
		_ => false,
	};
	match node {
		Node::Key(function, op @ (Op::Sub | Op::Hash), argument)
			if is_function(&function, op) && !(op == Op::Sub && is_spaced_minus(&function)) =>
		{
			let prefix = if op == Op::Sub { Op::Neg } else { Op::Hash };
			let argument = Node::Key(
				Box::new(Node::Empty),
				prefix,
				Box::new(negate_calls(*argument, functions, bound)),
			);
			Node::List(vec![*function, argument], Bracket::Round, Separator::None)
		}
		other => children_rewritten(other, |child| negate_calls(child, functions, bound)),
	}
}

/// A built-in word that applies to the value after it: a real function, print, a rounding word, a type, a library word of one value
fn is_builtin_callable(name: &str) -> bool {
	crate::real::FUNCTIONS.contains(&name)
		|| PRINT_WORDS.contains(&name)
		|| crate::wasm_emitter::ROUNDING_FUNCTIONS.contains(&name)
		|| super::is_type_call_word(name)
		|| crate::library_words::is_operand_word(name)
}

/// The name before a minus written apart (warp_parser::SpacedMinus)
fn is_spaced_minus(operand: &Node) -> bool {
	match operand {
		Node::Meta { node, data } => {
			matches!(data.as_ref(), Node::Data(dada) if dada.downcast_ref::<crate::warp_parser::SpacedMinus>().is_some())
				|| is_spaced_minus(node)
		}
		_ => false,
	}
}

/// A lifted lambda is never called by name (closures.rs): its captured values take the kinds of the variables captured (a
/// parameter of the enclosing function, a literal variable), its parameters the kinds of the literal arguments of the closure
/// calls of its arity
pub(super) fn infer_closure_parameters(ctx: &mut Context, program: &Node) {
	let variable_kinds = variable_kinds(program, ctx, true);
	loop {
		let mut inferred: Vec<(String, usize, Kind)> = vec![];
		// a closure call may call any lifted lambda of its arity: the kinds its arguments pass to each one
		let mut passed: HashMap<(String, usize), HashSet<Kind>> = HashMap::new();
		// the kinds of each scope's parameters and variables: `m = float(2.5)` captured by a closure is a float
		let function_kinds: HashMap<String, Kind> = ctx
			.user_functions
			.iter()
			.map(|(name, function)| (name.clone(), function.return_kind))
			.collect();
		let scope_kinds = |params: &[Param], body: &Node| -> HashMap<String, Kind> {
			let scope = function_body_scope(
				params,
				body,
				&function_kinds,
				&HashMap::new(),
				&ctx.closure_variable_targets,
			);
			scope
				.locals
				.into_iter()
				.map(|(name, local)| (name, local.kind))
				.collect()
		};
		let scopes: Vec<(&Node, HashMap<String, Kind>)> = std::iter::once((program, scope_kinds(&[], program)))
			.chain(
				ctx.user_functions
					.values()
					.map(|function| (function.body.as_ref(), scope_kinds(&function.params, &function.body))),
			)
			.collect();
		for (body, kinds) in scopes {
			body.visit(&mut |node| {
				let value_kind = |value: &Node| match value.drop_meta() {
					Node::Symbol(name) => kinds.get(name).or(variable_kinds.get(name)).copied(),
					// a cell's value is a Node of any kind (a signal's new value, lowering/signal_values.rs)
					Node::List(items, _, _)
						if items
							.first()
							.is_some_and(|word| word.drop_meta().name() == crate::wasm_emitter::cells::CELL_GET) =>
					{
						Some(Kind::Empty)
					}
					_ => argument_literal_kind(value),
				};
				if let Some((target, captured)) = crate::closures::as_closure_new(node) {
					inferred.extend(
						captured
							.iter()
							.enumerate()
							.filter_map(|(index, value)| Some((target.to_string(), index, value_kind(value)?))),
					);
				}
				let Node::List(items, _, _) = node else { return };
				let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) else {
					return;
				};
				let Some(arity) = crate::closures::closure_call_arity(name) else {
					return;
				};
				for (target, captured) in ctx.closure_targets.iter().filter(|(target, captured)| {
					ctx.user_functions
						.get(target)
						.is_some_and(|function| function.params.len() == captured + arity)
				}) {
					for (index, argument) in items[2..].iter().enumerate() {
						// an Int is certain only as a literal: an unknown variable reads as one too
						let kind = value_kind(argument)
							.filter(|kind| *kind != Kind::Int || argument_literal_kind(argument) == Some(Kind::Int));
						passed
							.entry((target.clone(), captured + index))
							.or_default()
							.extend(kind);
					}
				}
			});
		}
		inferred.extend(
			passed
				.into_iter()
				.filter_map(|((target, index), kinds)| Some((target, index, passed_kind(&kinds)?))),
		);
		let mut changed = false;
		for (target, index, kind) in inferred {
			let Some(param) = ctx
				.user_functions
				.get_mut(&target)
				.and_then(|function| function.params.get_mut(index))
			else {
				continue;
			};
			if kind != Kind::Int && takes_passed_kind(param) && param.used_as.is_none() {
				param.used_as = Some(kind);
				changed = true;
			}
		}
		if !changed {
			return;
		}
	}
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
	let article = if name.starts_with(|first: char| "AEIOUaeiou".contains(first)) {
		"an"
	} else {
		"a"
	};
	format!("{article} {name}")
}

/// The kind a call argument certainly has, judged from the literal alone
pub fn argument_literal_kind(argument: &Node) -> Option<Kind> {
	match argument.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::List(_, Bracket::Square, _) => {
			Some(infer_type(argument, &Scope::new()))
		}
		Node::Empty => Some(Kind::List),
		// `"" + n`, `"#" + x + y`: a concatenation with a text is a text
		Node::Key(_, Op::Add, _) if super::declaration_lowering::is_text(argument) => Some(Kind::Text),
		// `1.5 as float`, `x as float`: the kind the conversion names
		Node::Key(_, Op::As, target) if builtin_type_kind(&target.name()).is_some_and(|kind| kind.is_float()) => {
			Some(Kind::Float)
		}
		_ if crate::closures::as_closure_new(argument).is_some() => Some(Kind::Function),
		Node::List(entries, Bracket::Curly, _)
			if entries
				.iter()
				.all(|entry| matches!(entry.drop_meta(), Node::Key(_, Op::Colon, _))) =>
		{
			Some(Kind::List)
		} // a map
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
	for param in ctx
		.user_functions
		.values()
		.flat_map(|function| &function.params)
		.filter(|_| !unknown_keeps_kind)
	{
		kinds.insert(param.name.clone(), None);
	}
	let mut literal: HashSet<String> = HashSet::new();
	program.visit(&mut |node| {
		let Node::Key(target, Op::Assign | Op::Define, value) = node else {
			return;
		};
		let Node::Symbol(name) = target.drop_meta() else { return };
		let kind = argument_literal_kind(value);
		if kind.is_none() && unknown_keeps_kind {
			return;
		}
		literal.insert(name.clone());
		kinds
			.entry(name.clone())
			.and_modify(|known| {
				if *known != kind {
					*known = None
				}
			})
			.or_insert(kind);
	});
	kinds
		.into_iter()
		.filter(|(name, _)| !unknown_keeps_kind || literal.contains(name))
		.filter_map(|(name, kind)| Some((name, kind?)))
		.collect()
}

/// A decimal literal, `2.5` or `2.0` (an exact decimal, so its kind may be Int): a float an int parameter refuses (P49, P49b)
pub(super) fn is_decimal_literal(node: &Node) -> bool {
	matches!(
		node.drop_meta(),
		Node::Number(crate::extensions::numbers::Number::Float(_))
	)
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
	let variable_kinds = literal_variable_kinds(program, ctx);
	let argument_kind = |argument: &Node| {
		argument_literal_kind(argument).or_else(|| match argument.drop_meta() {
			Node::Symbol(name) => variable_kinds.get(name).copied(),
			_ => None,
		})
	};
	let mut argument_kinds: HashMap<(String, usize), Vec<Kind>> = HashMap::new();
	// is every argument a parameter is given a bool (`show(1<2)`, `show(ok)` of `ok = x > 2`)
	let bool_variables = bool_variables(program);
	let is_bool_argument = |argument: &Node| {
		crate::analyzer::is_boolean(argument, &Scope::new())
			|| matches!(argument.drop_meta(), Node::Symbol(name) if bool_variables.contains(name))
	};
	let mut given_bools: HashMap<(String, usize), bool> = HashMap::new();
	// a float passed to a declared int parameter loses digits: a compile error (P49)
	let mut float_for_int: Vec<String> = Vec::new();
	// a parameter of the same name shadows the variable (as in literal_variable_kinds)
	let mut decimal_variables = decimal_variables(program);
	ctx.user_functions
		.values()
		.flat_map(|function| &function.params)
		.for_each(|param| {
			decimal_variables.remove(&param.name);
		});
	let has_digits_an_int_loses = |argument: &Node| {
		is_decimal_literal(argument)
			|| argument_kind(argument) == Some(Kind::Float)
			|| matches!(argument.drop_meta(), Node::Symbol(name) if decimal_variables.contains(name))
	};
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
				// the head `show(b)` and a recursive `show(b)` pass the parameter itself: no say in its kind
				if !matches!(argument.drop_meta(), Node::Symbol(passed) if *passed == function.params[index].name) {
					*given_bools.entry((name.clone(), index)).or_insert(true) &= is_bool_argument(argument);
				}
				// `real`, `exact` are Ints that may hold a ratio: only a whole type loses a decimal's digits
				let declared = function.params[index]
					.annotation
					.as_ref()
					.map(|annotation| annotation.name())
					.filter(|declared| crate::analyzer::checks::refuses_decimals(declared));
				if let Some(declared) = declared.filter(|_| has_digits_an_int_loses(argument)) {
					// a whole float serializes as `2`; its decimal point is what makes it no int
					let written = match argument.drop_meta() {
						Node::Number(crate::extensions::numbers::Number::Float(value)) if value.fract() == 0.0 => {
							format!("{value:.1}")
						}
						other => other.serialize(),
					};
					float_for_int.push(format!("{written} is no {declared}: write {written} as int"));
				}
				// `g("a")`: a one-character text is parsed as a Codepoint; the parameter holds any text, as for a default
				if let Some(kind) =
					argument_kind(argument).map(|kind| if kind == Kind::Codepoint { Kind::Text } else { kind })
				{
					let kinds = argument_kinds.entry((name.clone(), index)).or_default();
					if !kinds.contains(&kind) {
						kinds.push(kind);
					}
				}
			}
		}
	});
	ctx.parameter_conflicts.extend(float_for_int);
	// given a bool at every call, an untyped parameter is one, as if written `b: bool`: it keeps printing yes/no
	// (card bool-loses: `show(b) := "got " + b; show(1<2)` gave "got 1")
	for ((name, index), _) in given_bools.into_iter().filter(|(_, all_bools)| *all_bools) {
		let param = &mut ctx
			.user_functions
			.get_mut(&name)
			.expect("call sites were collected from known functions")
			.params[index];
		if param.annotation.is_none() && param.default.is_none() {
			param.annotation = Some(Node::Symbol(crate::analyzer::BOOL_TYPE.to_string()));
		}
	}
	// a value its body gives the parameter is one more kind it holds; with no call of known kind it starts as an Int
	for (name, function) in &ctx.user_functions {
		for (index, param) in function.params.iter().enumerate() {
			let assigned = assigned_literal_kinds(&function.body, &param.name);
			if !assigned.is_empty() {
				let kinds = argument_kinds
					.entry((name.clone(), index))
					.or_insert_with(|| vec![param.used_as.unwrap_or(Kind::Int)]);
				kinds.extend(
					assigned
						.into_iter()
						.filter(|kind| !kinds.contains(kind))
						.collect::<Vec<_>>(),
				);
			}
		}
	}
	for ((name, index), kinds) in argument_kinds {
		let function = ctx
			.user_functions
			.get_mut(&name)
			.expect("call sites were collected from known functions");
		let param = &mut function.params[index];
		if param.annotation.is_some() || param.default.is_some() {
			continue;
		}
		match kinds.as_slice() {
			[Kind::Int] => {}
			[kind] => param.used_as = Some(*kind),
			// called with several kinds, it takes any value, as if written `x:any` (P173: silently, no annotation asked)
			[_, _, ..] => param.annotation = Some(Node::Symbol(crate::type_kinds::UNTYPED_FIELD.to_string())),
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
	let mut function_kinds: HashMap<String, Kind> = ctx
		.user_functions
		.keys()
		.map(|name| (name.clone(), Kind::Int))
		.collect();
	function_kinds.extend(ctx.field_kinds.clone());
	let mut stale: Vec<&UserFunctionDef> = ctx.user_functions.values().collect();
	for _ in 0..=ctx.user_functions.len() {
		let inferred: Vec<(String, Kind)> = stale
			.iter()
			.map(|function| {
				let kind =
					crate::closures::closure_call_kind(&function.name, ctx, &function_kinds).unwrap_or_else(|| {
						infer_function_return_kind(
							&function.params,
							&function.body,
							&function_kinds,
							globals,
							&ctx.closure_variable_targets,
						)
					});
				// `if t == ø { return [] }; return f(t.left) + [x]`: assumed to return an Int, the recursion makes an
				// error of the last value; assumed to return a Node of unknown kind, it settles
				let kind = if kind == Kind::Error && function_kinds.get(&function.name) == Some(&Kind::Int) {
					let mut unknown_recursion = function_kinds.clone();
					unknown_recursion.insert(function.name.clone(), Kind::Empty);
					infer_function_return_kind(
						&function.params,
						&function.body,
						&unknown_recursion,
						globals,
						&ctx.closure_variable_targets,
					)
				} else {
					kind
				};
				(function.name.clone(), kind)
			})
			.collect();
		let tuples: Vec<(String, String, Kind)> = stale
			.iter()
			.flat_map(|function| {
				let kinds = infer_tuple_kinds(
					&function.params,
					&function.body,
					&function_kinds,
					globals,
					&ctx.closure_variable_targets,
				);
				kinds
					.into_iter()
					.enumerate()
					.map(|(index, kind)| {
						(
							function.name.clone(),
							crate::tuples::element_key(&function.name, index),
							kind,
						)
					})
					.collect::<Vec<_>>()
			})
			.collect();
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
		let changed: Vec<&str> = changed.iter().map(String::as_str).collect();
		stale = ctx
			.user_functions
			.values()
			.filter(|function| {
				crate::closures::closure_call_kind(&function.name, ctx, &function_kinds).is_some()
					|| function.body.mentions_any(&changed)
			})
			.collect();
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
