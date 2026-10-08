//! Variables and scope: collecting a program's variables, main-level bindings, the Scope of locals

use super::*;
use std::collections::HashSet;

/// Collect variables defined in node and populate scope
/// Returns count of temp locals needed (e.g., for while loops)
pub fn collect_variables(node: &Node, scope: &mut Scope) -> u32 {
	let before = scope.clone();
	let temporaries = collect_variables_inner(node, scope, false, false);
	// `b = 0; global b = 1`: a variable written before its `global` declaration is that global throughout, never a
	// local of the same name besides it; collected again with it global from the start
	let split: Vec<Local> = scope.globals.values().filter(|global| scope.locals.contains_key(&global.name) && !before.locals.contains_key(&global.name)).cloned().collect();
	if split.is_empty() {
		return temporaries;
	}
	*scope = before;
	scope.globals.extend(split.into_iter().map(|global| (global.name.clone(), global)));
	collect_variables_inner(node, scope, false, false)
}

/// The binding a `global` declaration introduces: `global x` (an Int) and `global x=7`; it has no local slot
pub(super) fn global_binding(declaration: &Node, scope: &Scope) -> Option<Local> {
	let (name, value) = match declaration.drop_meta() {
		Node::Symbol(name) => (name, &Node::Number(Number::Int(0))),
		Node::Key(target, Op::Assign | Op::Define, value) => match target.drop_meta() {
			Node::Symbol(name) => (name, value.as_ref()),
			_ => return None,
		},
		_ => return None,
	};
	let (kind, type_node) = value_binding(value, scope);
	Some(Local { type_node, ..Local::new(0, name.clone(), kind) })
}

/// Kind and type of a variable bound to `value` without a declared type; a list keeps its element type
pub(super) fn value_binding(value: &Node, scope: &Scope) -> (Kind, Option<Box<Node>>) {
	let kind = binding_kind(value, scope);
	let type_name = match kind {
		Kind::List => Some(list_type_name(value, scope)),
		Kind::Int if is_boolean(value, scope) => Some(BOOL_TYPE.to_string()),
		_ => None,
	};
	(kind, type_name.map(|name| Box::new(Node::Symbol(name))))
}

/// The body of a function definition `f(x) = …`, `f(x) := …`, `def f(x) {…}`: its variables are the function's own
pub(super) fn function_definition_body(node: &Node) -> Option<&Node> {
	let starts_with_symbol = |items: &[Node]| matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_)));
	match node.drop_meta() {
		Node::Key(left, Op::Assign | Op::Define, body) if matches!(left.drop_meta(), Node::List(items, _, _) if starts_with_symbol(items)) => Some(body),
		Node::List(items, _, _) if items.len() >= 2 && matches!(items[0].drop_meta(), Node::Symbol(keyword) if is_function_keyword(keyword)) => items.last(),
		_ => None,
	}
}

pub(super) fn collect_variables_inner(node: &Node, scope: &mut Scope, skip_first_assign: bool, in_structure: bool) -> u32 {
	let node = node.drop_meta();
	if function_definition_body(node).is_some() {
		return 0;
	}
	if let Some((names, values)) = crate::tuples::destructuring(node) {
		let temporaries = values.iter().map(|value| collect_variables_inner(value, scope, false, in_structure)).sum();
		// with a starred name the values are unpacked from one list: each name holds a Node, the star a list
		let starred = names.iter().any(|name| crate::tuples::unstarred(name) != name);
		for (index, name) in names.iter().enumerate() {
			let rest = crate::tuples::unstarred(name);
			if scope.lookup(rest).is_none() && !scope.is_global(rest) {
				let kind = match (starred, rest != name) {
					(_, true) => Kind::List,
					(true, false) => Kind::Data,
					(false, false) => destructured_kind(values, index, scope),
				};
				scope.define(rest.to_string(), None, kind);
			}
		}
		return temporaries;
	}
	match node {
		// `c ? a : b`: both branches are code, never a tag `a:b` whose body holds attributes
		Node::Key(condition, Op::Question, then_else) if matches!(then_else.drop_meta(), Node::Key(_, Op::Colon, _)) => {
			let Node::Key(then, _, otherwise) = then_else.drop_meta() else { unreachable!("guarded") };
			collect_variables_inner(condition, scope, false, in_structure)
				+ collect_variables_inner(then, scope, false, in_structure)
				+ collect_variables_inner(otherwise, scope, false, in_structure)
		}
		// Global declarations: global:Key(name, =, value) - don't create local
		// Tag structures: html:body - body is structure context (attributes, not variables)
		Node::Key(left, Op::Colon, right) => {
			if let Node::Symbol(kw) = left.drop_meta() {
				if kw == "global" {
					// Don't define local for global variable
					// But still count any variables in the value expression, first: the kind of a hoisted block
					// `global:xs = (made = ø; …; made)` is the kind of its temporaries
					let count = collect_variables_inner(right, scope, true, false);
					if let Some(global) = global_binding(right, scope).filter(|global| !scope.is_global(&global.name)) {
						scope.globals.insert(global.name.clone(), global);
					}
					return count;
				}
				// Symbol:body is a tag/structure - right side is structure context
				// Inside structures, Op::Assign is attribute, not variable
				return collect_variables_inner(left, scope, false, in_structure)
					+ collect_variables_inner(right, scope, false, true);
			}
			collect_variables_inner(left, scope, false, in_structure) + collect_variables_inner(right, scope, false, in_structure)
		}
		// Define (:=) always creates a variable, Assign (=) only outside structure context
		Node::Key(left, Op::Define, right) => {
			if !skip_first_assign {
				if let Node::Symbol(name) = left.drop_meta() {
					if scope.lookup(name).is_none() && !scope.is_global(name) {
						let kind = binding_kind(right, scope);
						scope.define(name.clone(), None, kind);
					}
				}
			}
			collect_variables_inner(right, scope, false, in_structure)
		}
		// Assign creates variables only at top level (not inside structures)
		Node::Key(left, Op::Assign, right) => {
			// the variables the value binds come first, so its kind is known: `a = (t = 1 + 2; t)` makes a an Int
			let inner_temporaries = collect_variables_inner(right, scope, false, in_structure);
			if !skip_first_assign && !in_structure {
				// Check for typed declaration: Key(Key(name, Colon, type), Assign, value)
				match left.drop_meta() {
					// `k.x = 3` updates k: it binds no k, so an undefined k stays undefined (as in `k#1 = 3`)
					Node::Symbol(name) if crate::library_words::is_field_update_of(name, right) && scope.lookup(name).is_none() => {}
					Node::Symbol(name) if scope.lookup(name).is_none() && !scope.is_global(name) => {
						let declared = declared_type(left);
						let (kind, type_node) = match declared {
							Some(type_name) => (declared_kind(&type_name.name()).unwrap_or_else(|| binding_kind(right, scope)), Some(Box::new(type_name.clone()))),
							None => value_binding(right, scope),
						};
						let is_declared = declared.is_some();
						scope.define(name.clone(), type_node, kind);
						if let Some(local) = scope.own_binding_mut(name) {
							local.declared = is_declared;
						}
					}
					Node::Symbol(name) => {
						type_list_by_first_append(name, right, scope);
						widen_list_type(scope, name, right);
						widen_to_float(scope, name, right);
						widen_to_node(scope, name, right);
					}
					Node::Key(list, Op::Hash, _) => widen_element_type(scope, list, right),
					// Typed variable: x:int = 1 parses as Key(Key(x, Colon, int), Assign, 1)
					Node::Key(var_name, Op::Colon, type_node) => {
						if let Node::Symbol(name) = var_name.drop_meta() {
							if scope.lookup(name).is_none() {
								// Get kind from type annotation
								let type_str = type_node.drop_meta().to_string();
								let kind = declared_kind(&type_str).unwrap_or(Kind::Int);
								scope.define(name.clone(), Some(type_node.clone()), kind);
							}
						}
					}
					_ => {}
				}
			}
			inner_temporaries
		}
		// Compound assignments don't create new variables
		Node::Key(left, op, right) if op.is_compound_assign() => {
			if let Node::Symbol(name) = left.drop_meta() {
				widen_to_float(scope, name, right);
				// `xs += [v]` (a lowered `xs.push(v)`) types xs as `xs = xs + [v]` does
				if *op == Op::AddAssign {
					type_list_by_first_append(name, &Node::Key(left.clone(), Op::Add, right.clone()), scope);
				}
			}
			collect_variables_inner(left, scope, false, in_structure) + collect_variables_inner(right, scope, false, in_structure)
		}
		Node::Key(left, Op::Do, right) => {
			// While loop needs temp locals for the result and for "the body ran"
			2 + collect_variables_inner(left, scope, false, in_structure) + collect_variables_inner(right, scope, false, in_structure)
		}
		Node::Key(left, Op::Abs, right) if matches!(left.drop_meta(), Node::Empty) => {
			// Integer abs needs a temp local for the if-then-else pattern
			1 + collect_variables_inner(right, scope, false, in_structure)
		}
		Node::Key(left, _, right) => {
			collect_variables_inner(left, scope, false, in_structure) + collect_variables_inner(right, scope, false, in_structure)
		}
		// a group of statements in a structure is code: `ul{ (made = ø; for … ; made) }`, a lowered `[li{t} for t in ts]`
		Node::List(items, Bracket::Round, Separator::Semicolon | Separator::Newline) => {
			items.iter().map(|item| collect_variables_inner(item, scope, false, false)).sum()
		}
		// `ran_without_abort(i, {…})` keeps the try depth of its start in a temp local (try_guard.rs)
		Node::List(items, Bracket::Round, Separator::None) if items.first().is_some_and(|head| matches!(head.drop_meta(), Node::Symbol(name) if name == crate::wasm_emitter::RAN_WITHOUT_ABORT)) => {
			1 + items.iter().map(|item| collect_variables_inner(item, scope, false, in_structure)).sum::<u32>()
		}
		Node::List(items, _, _) => {
			items.iter().map(|item| collect_variables_inner(item, scope, false, in_structure)).sum()
		}
		_ => 0,
	}
}

/// `xs = []; xs.push(5)`, lowered to `xs = xs + [5]`: the empty list takes the type of its first appended elements
pub(super) fn type_list_by_first_append(name: &str, value: &Node, scope: &mut Scope) {
	let Node::Key(list, Op::Add, appended) = value.drop_meta() else { return };
	let appends_to_itself = matches!(list.drop_meta(), Node::Symbol(target) if target == name);
	if !appends_to_itself || !matches!(appended.drop_meta(), Node::List(_, Bracket::Square, _)) {
		return;
	}
	let appended_type = list_type_name(appended, scope);
	if let Some(local) = scope.own_binding_mut(name).filter(|local| local.kind == Kind::Empty && local.type_node.is_none()) {
		local.kind = Kind::List;
		local.type_node = Some(Box::new(Node::Symbol(appended_type)));
	}
}

/// `xs#i = v` with an element of another type than the inferred `list of <type>` of xs: a list of ints given a float
/// holds floats (`xs = [0, 0]; xs#1 = random()`), any other mix Nodes of any type; a declared type is kept (checked elsewhere)
pub(super) fn widen_element_type(scope: &mut Scope, list: &Node, value: &Node) {
	let Node::Symbol(name) = list.drop_meta() else { return };
	let assigned = element_type_word(value, scope);
	let Some(local) = scope.own_binding_mut(name) else { return };
	let Some(element) = local.type_node.as_ref().and_then(|type_node| type_node.name().strip_prefix(LIST_OF_PREFIX).map(str::to_string)) else { return };
	let widened = match (element.as_str(), assigned.as_str()) {
		(element, assigned) if element == assigned => return,
		(INT_WORD, FLOAT_WORD) => format!("{LIST_OF_PREFIX}{FLOAT_WORD}"),
		(_, INT_WORD) if element == RATIONAL_WORD || element == FLOAT_WORD => return, // an Int fits a rational or float list
		(FLOAT_WORD, RATIONAL_WORD) => return, // `xs#1 = 0.5` (an exact decimal) of a float list is stored as its f64
		_ => NODE_LIST_TYPE.to_string(),
	};
	local.type_node = Some(Box::new(Node::Symbol(widened)));
}

/// `ys = ["a"]; ys = [3]`: a variable given lists of two element types holds the items of both, of their common type
/// (`list of number` for ints and floats), else held as Nodes; a declared type is kept (checked elsewhere)
fn widen_list_type(scope: &mut Scope, name: &str, value: &Node) {
	if infer_type(value, scope) != Kind::List {
		return;
	}
	let assigned = list_type_name(value, scope);
	let Some(local) = scope.own_binding_mut(name).filter(|local| local.kind == Kind::List && !local.is_param) else { return };
	let Some(held) = local.type_node.as_ref().map(|type_node| type_node.name()) else { return };
	let (Some(held_element), Some(assigned_element)) = (held.strip_prefix(LIST_OF_PREFIX), assigned.strip_prefix(LIST_OF_PREFIX)) else { return };
	if held_element == assigned_element {
		return;
	}
	let common = super::checks::common_type_word(&[held_element.to_string(), assigned_element.to_string()]);
	let widened = common.map_or(NODE_LIST_TYPE.to_string(), |element| format!("{LIST_OF_PREFIX}{element}"));
	local.type_node = Some(Box::new(Node::Symbol(widened)));
}

/// An exact variable that is later assigned an f64 (`x=10; x=floor(2.5)` with libm's floor) holds an f64 throughout,
/// as an expression mixing in an f64 is one; a declared type is kept (and checked elsewhere). A global too: `b = 0.5`
/// changed by a function's `global b; b = b + random()`
pub(super) fn widen_to_float(scope: &mut Scope, name: &str, value: &Node) {
	let float_value = infer_type(value, scope).is_float();
	let binding = match scope.locals.contains_key(name) {
		true => scope.locals.get_mut(name),
		false => scope.globals.get_mut(name),
	};
	if let Some(local) = binding.filter(|local| local.kind == Kind::Int && local.type_node.is_none()) {
		if float_value {
			local.kind = Kind::Float;
		}
	}
}

/// The kinds a variable may hold whose values are told apart only by representation; a variable given values of two
/// of them that do not mix (a list, then an Int; a text, then an Int) is a compile error when both kinds are evident
/// from the source (P45, check_kind_changes); where the compiler only infers them it is held as a Node of run-time kind.
/// Int and Float widen to Float instead (widen_to_float), a text and a character are both texts.
pub(super) const CONCRETE_KINDS: [Kind; 6] = [Kind::Int, Kind::Float, Kind::Text, Kind::Codepoint, Kind::List, Kind::Symbol];

pub(super) fn widen_to_node(scope: &mut Scope, name: &str, value: &Node) {
	let assigned = binding_kind(value, scope);
	// a parameter's representation comes from its calls (infer_parameters_from_calls), not from the body
	let Some(local) = scope.locals.get_mut(name).filter(|local| !local.is_param && local.type_node.as_ref().is_none_or(|_| local.kind == Kind::List)) else { return };
	let mixes = |a: Kind, b: Kind| a == b || [a, b].iter().all(|kind| matches!(kind, Kind::Int | Kind::Float)) || [a, b].iter().all(|kind| matches!(kind, Kind::Text | Kind::Codepoint));
	// an element of unknown kind may be an object: `item = 2` earlier, then `for item in basket` (samples/natural.warp)
	let unknown_element = assigned == Kind::Empty && matches!(value.drop_meta(), Node::Key(_, Op::Hash, _));
	let other_kind = unknown_element || CONCRETE_KINDS.contains(&assigned) && !mixes(local.kind, assigned);
	if CONCRETE_KINDS.contains(&local.kind) && other_kind {
		local.kind = Kind::Empty;
		local.type_node = None;
	}
}

/// The kind a value evidently has as written: a literal, a list literal, arithmetic of numbers, or a name or call whose
/// kind `known` gives; None for anything else (a variable, a loop variable), whose kind only inference knows
pub(super) fn evident_kind(value: &Node, known: &HashMap<String, Kind>) -> Option<Kind> {
	match value.drop_meta() {
		Node::Number(Number::Float(_)) => Some(Kind::Float),
		Node::Number(_) => Some(Kind::Int),
		Node::Text(_) | Node::Char(_) => Some(Kind::Text),
		Node::List(_, Bracket::Square, _) => Some(Kind::List),
		Node::Key(left, op, right) if op.is_arithmetic() => match (evident_kind(left, known)?, evident_kind(right, known)?) {
			(Kind::Int, Kind::Int) => Some(Kind::Int),
			(Kind::Int | Kind::Float, Kind::Int | Kind::Float) => Some(Kind::Float),
			_ => None,
		},
		Node::Symbol(name) => known.get(name).copied(),
		Node::List(items, Bracket::Round, Separator::None) => match items.first().map(Node::drop_meta) {
			Some(Node::Symbol(function)) => known.get(&format!("{function}{CALL_SUFFIX}")).copied(),
			_ => None,
		},
		_ => None,
	}
}

/// Marks a function's result in the `known` kinds of evident_kind, apart from a variable of the same name
const CALL_SUFFIX: &str = "()";

/// The result kind of each user function whose last expression's kind is evident, its declared parameters known:
/// `f(x: int) := x + 1` gives an Int, `f() := "a"` a Text
fn evident_results(program: &Node) -> HashMap<String, Kind> {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, program);
	ctx.user_functions.values().filter_map(|function| {
		let parameters = function.params.iter().filter_map(|param| {
			let kind = builtin_type_kind(annotated_builtin_type(param.annotation.as_ref()?)?)?;
			Some((param.name.clone(), if kind == Kind::Codepoint { Kind::Text } else { kind }))
		}).collect();
		let kind = evident_kind(last_expression(&function.body), &parameters)?;
		Some((format!("{}{CALL_SUFFIX}", function.name), kind))
	}).collect()
}

/// The expression a body gives: its last statement
fn last_expression(body: &Node) -> &Node {
	match body.drop_meta() {
		Node::List(items, Bracket::Curly | Bracket::None, Separator::Semicolon | Separator::Newline) if !items.is_empty() => last_expression(items.last().expect("not empty")),
		Node::List(items, Bracket::Curly, _) if items.len() == 1 => last_expression(&items[0]),
		other => other,
	}
}

/// P45 (user: "compile error unless we are in script mode, which is not defined yet"): a variable given values of
/// evidently different kinds that do not mix, `x = [1]; x = 5`, in the program or in one function body
pub fn check_kind_changes(program: &Node) -> Option<Diagnostic> {
	let mut bodies = vec![program];
	program.visit(&mut |node| {
		if let Node::Key(_, Op::Define, body) = node {
			bodies.push(body);
		}
	});
	let results = evident_results(program);
	let globals = global_variables(program);
	bodies.into_iter().find_map(|body| first_kind_change(body, &mut HashMap::new(), &results, &globals))
}

/// The variables declared `global y` (at this stage late_binding has moved a function's `global y` to main)
fn global_variables(program: &Node) -> HashSet<String> {
	let mut globals = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Key(keyword, Op::Colon, declared) = node.drop_meta() {
			if keyword.drop_meta().name() == crate::late_binding::GLOBAL {
				let target = if let Node::Key(target, Op::Assign, _) = declared.drop_meta() { target } else { declared };
				globals.insert(target.name());
			}
		}
	});
	globals
}

/// The evident kind of a value for P45's kind changes, with bools apart from ints: int ≰ bool, bool ≤ int (card bool-assign)
#[derive(Clone, Copy, PartialEq)]
pub(super) enum EvidentKind {
	Of(Kind),
	Bool,
	/// a name a function definition binds, `f = x => x*2` lowered to `(f x) := x*2` included
	Function,
}

impl EvidentKind {
	fn of(value: &Node, results: &HashMap<String, Kind>) -> Option<EvidentKind> {
		match value.drop_meta() {
			Node::True | Node::False => Some(EvidentKind::Bool),
			Node::Key(_, op, _) if op.is_comparison() => Some(EvidentKind::Bool),
			Node::Key(_, Op::Arrow | Op::FatArrow, _) => Some(EvidentKind::Function),
			_ => evident_kind(value, results).map(EvidentKind::Of),
		}
	}

	/// The kind a variable that was `self` has after it is given `now`, None when `now` does not mix with it
	fn given(self, now: EvidentKind) -> Option<EvidentKind> {
		use EvidentKind::{Bool, Function, Of};
		match (self, now) {
			(Bool, Bool) => Some(Bool),
			(Function, Function) => Some(Function),
			(Of(Kind::Int | Kind::Float), Bool) => Some(self),
			(Of(was), Of(now)) if was == now => Some(self),
			(Of(Kind::Int | Kind::Float), Of(Kind::Int | Kind::Float)) => Some(if now == Of(Kind::Float) { now } else { self }),
			_ => None,
		}
	}

	fn with_article(self) -> String {
		match self {
			EvidentKind::Bool => "a Bool".to_string(),
			EvidentKind::Function => "a function".to_string(),
			EvidentKind::Of(kind) => kind_with_article(kind),
		}
	}
}

/// Walks the assignments in program order, not into function definitions or lambdas (their own scopes);
/// `kinds` holds each variable's evident kind, None once it was given a value of no evident kind
pub(super) fn first_kind_change(node: &Node, kinds: &mut HashMap<String, Option<EvidentKind>>, results: &HashMap<String, Kind>, globals: &HashSet<String>) -> Option<Diagnostic> {
	match node.drop_meta() {
		Node::Key(head, Op::Define, value) => match head.drop_meta() {
			// `(f x) := …`: f names a function from here on; its body gives the variables it shares, the globals and
			// its `nonlocal` ones, values of their kinds (card nonlocal-assign)
			Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => {
				given_kind(&items[0].name(), Some(EvidentKind::Function), value, kinds).or_else(|| {
					let nonlocals = crate::late_binding::declared_nonlocals(value);
					let is_shared = |name: &String| globals.contains(name) || nonlocals.contains(name);
					let mut body_kinds: HashMap<_, _> = kinds.iter().filter(|(name, _)| is_shared(name)).map(|(name, kind)| (name.clone(), *kind)).collect();
					let change = first_kind_change(value, &mut body_kinds, results, globals);
					kinds.extend(body_kinds.into_iter().filter(|(name, _)| is_shared(name)));
					change
				})
			}
			_ => None,
		},
		Node::Key(_, Op::Arrow | Op::FatArrow, _) => None,
		Node::Key(target, Op::Assign, value) => {
			if let Some(change) = first_kind_change(value, kinds, results, globals) {
				return Some(change);
			}
			let Node::Symbol(name) = target.drop_meta() else { return None };
			given_kind(name, EvidentKind::of(value, results), value, kinds)
		}
		Node::Key(left, _, right) => first_kind_change(left, kinds, results, globals).or_else(|| first_kind_change(right, kinds, results, globals)),
		Node::List(items, _, _) => items.iter().find_map(|item| first_kind_change(item, kinds, results, globals)),
		_ => None,
	}
}

/// The variable `name` is given `value` of the evident kind `given`: the kinds it holds from here on, or the change
fn given_kind(name: &str, given: Option<EvidentKind>, value: &Node, kinds: &mut HashMap<String, Option<EvidentKind>>) -> Option<Diagnostic> {
	// a first evident value fixes the kind (Int widens to Float); any value of no evident kind ends the check
	let kind = match (kinds.get(name).copied(), given) {
		(None, given) => given,
		// P199: 1 and 0 are yes and no, a bool variable stays one
		(Some(Some(EvidentKind::Bool)), Some(_)) if crate::analyzer::is_zero_or_one(value) => Some(EvidentKind::Bool),
		(Some(Some(was)), Some(now)) => match was.given(now) {
			Some(kind) => Some(kind),
			None => {
				let message = format!("{name} was {}, is given {}: use another name", was.with_article(), now.with_article());
				return Some(Diagnostic::at(value, message));
			}
		},
		_ => None,
	};
	kinds.insert(name.to_string(), kind);
	None
}

/// Kind of a new variable bound to `value`: `x=ø` makes x an optional, held as a Node that is ø until assigned
/// The kind of the `index`-th name of `x, y = …`: the tuple function's value kind, or that of the value in that position
pub(super) fn destructured_kind(values: &[Node], index: usize, scope: &Scope) -> Kind {
	match values {
		[call] => crate::tuples::call_parts(call)
			.and_then(|(function, _)| scope.function_kinds.get(&crate::tuples::element_key(function, index)).copied())
			.unwrap_or(Kind::Data),
		_ => values.get(index).map_or(Kind::Data, |value| binding_kind(value, scope)),
	}
}

pub(super) fn binding_kind(value: &Node, scope: &Scope) -> Kind {
	held_kind(value, || infer_type(value, scope))
}

/// Kind of a variable or global holding `value`, whose expression kind is `inferred`
pub(crate) fn held_kind(value: &Node, inferred: impl FnOnce() -> Kind) -> Kind {
	match value.drop_meta() {
		Node::Empty => Kind::Empty,
		Node::List(items, Bracket::Curly, _) if items.is_empty() => Kind::Empty, // the empty block is ø as well
		Node::Char(_) => Kind::Text, // a variable holding "a" may later hold "ab": one-character texts are held as nodes
		// so does a copy of a character: `f(t) { n = t; n = prev[n] }` called with `f("F")`
		Node::Symbol(_) => match inferred() {
			Kind::Codepoint => Kind::Text,
			kind => kind,
		},
		_ => inferred(),
	}
}

/// Kind of a variable declared `x:T`; an optional `T?` may hold ø, so it is held as a Node
pub(super) fn declared_kind(type_name: &str) -> Option<Kind> {
	match type_name.strip_suffix('?') {
		Some(_) => Some(Kind::Empty),
		// an inline union `int or text` holds a value of either kind as a Node
		None if super::checks::union_parts(type_name).is_some() => Some(Kind::Empty),
		None => builtin_type_kind(type_name),
	}
}

/// Outer variables a function body reads: bound in `outer`, not a parameter or local of the body.
/// Functions capture these by value when they are defined (DESIGN.md: immutable local bindings).
pub fn captured_variables(function: &UserFunctionDef, outer: &Scope) -> Vec<(String, Kind)> {
	let mut own = Scope::new();
	for param in &function.params {
		own.define(param.name.clone(), None, param_kind(param));
	}
	collect_variables(&function.body, &mut own);
	for variable in crate::lowering::for_loop::loop_variables(&function.body) {
		own.define(variable, None, Kind::Empty);
	}
	let mut captured: Vec<(String, Kind)> = vec![];
	function.body.visit(&mut |node| {
		if let Node::Symbol(name) = node {
			let is_new = own.lookup(name).is_none() && !captured.iter().any(|(seen, _)| seen == name);
			if let Some(local) = outer.lookup(name).filter(|_| is_new) {
				captured.push((name.clone(), local.kind));
			}
		}
	});
	captured
}

/// A function may change a main-level variable only when it is declared `global` (wiki/effects.md: State effect).
/// Assigning one from a function would otherwise silently either shadow it (Python) or mutate it (JavaScript):
/// reading or updating it first (`n += 1`) is an error that educates; a fresh `n = value` is ambiguous and asks
/// (`local-or-global`). Names the user means as main's variable become `global` declarations of the program.
pub fn resolve_main_variable_assignments(program: Node) -> Result<Node, Node> {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, &program);
	let mut main = Scope::new();
	collect_variables(&program, &mut main);
	let mut functions: Vec<&UserFunctionDef> = ctx.user_functions.values().collect();
	functions.sort_by(|a, b| a.name.cmp(&b.name));
	let mut meant_global: Vec<String> = vec![];
	let blocks = block_function_names(&program);
	let mut outside_blocks = Scope::new(); // a variable a block binds for itself is no outer variable
	collect_variables(&without_block_bodies(program.clone(), &blocks), &mut outside_blocks);
	// a used module's functions never see the program's variables (card module-locals)
	for function in functions.into_iter().filter(|function| !crate::modules::is_module_definition(&function.name)) {
		let looped = crate::lowering::for_loop::loop_variables(&function.body);
		let is_main_variable = |name: &String| main.lookup(name).is_some() && !main.is_global(name)
			&& !function.params.iter().any(|param| param.name == *name) && !declares_local(&function.body, name) && !looped.contains(name);
		let mut decided: HashSet<&String> = HashSet::new();
		for (node, name) in find_changes(&function.body, &is_main_variable) {
			if !decided.insert(name) {
				continue;
			}
			if blocks.contains(&function.name) {
				if outside_blocks.lookup(name).is_some() {
					educate_block_assignment(node, name, &function.name);
				}
				continue;
			}
			if !starts_with_fresh_binding(&function.body, name) {
				return Err(Diagnostic::at(node, format!(
					"{name} is a main-level variable: declare it `global {name}` to change it from a function, or use a new local name"))
					.fix(format!("global {name}")).into_error());
			}
			if ask_local_or_global(node, name, &function.name, main_assignment(&program, name))? == MAIN_LEVEL_READING && !meant_global.contains(name) {
				meant_global.push(name.clone());
			}
		}
	}
	Ok(declare_global(program, &meant_global))
}

/// The functions defined as a block of statements, `inc := {x = x+1}`
pub(super) fn block_function_names(program: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Key(name, Op::Define, body) = node {
			if let (Node::Symbol(name), true) = (name.drop_meta(), is_statement_block(body)) {
				names.insert(name.clone());
			}
		}
	});
	names
}

pub(super) fn without_block_bodies(node: Node, blocks: &HashSet<String>) -> Node {
	match node {
		Node::Key(name, Op::Define, _) if matches!(name.drop_meta(), Node::Symbol(name) if blocks.contains(name)) => Node::Empty,
		Node::Key(left, op, right) => Node::Key(Box::new(without_block_bodies(*left, blocks)), op, Box::new(without_block_bodies(*right, blocks))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| without_block_bodies(item, blocks)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(without_block_bodies(*node, blocks)), data },
		other => other,
	}
}

/// A block captures the outer variables by value (user decision D7): its assignment changes the block's own copy,
/// so `x=1; inc:={x=x+1}; do inc; x` stays 1. Educate toward the forms that change x: `global x`, or returning the value.
pub(super) fn educate_block_assignment(assignment: &Node, name: &str, block: &str) {
	crate::normalize::set_position_of(assignment);
	let written = crate::normalize::operand_text(assignment);
	let reason = format!("the block {block} captures {name} by value: its change stays inside the block");
	crate::normalize::advise(&written, &format!("global {name}"), &reason);
	crate::normalize::advise(&written, &format!("{name} = {block}()"), "or return the value from the block and assign it");
}

pub(super) const LOCAL_OR_GLOBAL: &str = "local-or-global";
pub(super) const LOCAL_KEYWORDS: [&str; 2] = ["let", "var"];
pub(super) const MAIN_LEVEL_READING: usize = 1;

/// The main-level statement that first assigns `name`: `n=0` of `n=0; def f(x){n=5;x}`
pub(super) fn main_assignment<'a>(program: &'a Node, name: &str) -> Option<&'a Node> {
	let statements = match program.drop_meta() {
		Node::List(items, _, _) => items.as_slice(),
		_ => std::slice::from_ref(program),
	};
	statements.iter().find(|statement| matches!(statement.drop_meta(), Node::Key(target, Op::Assign, _) if matches!(target.drop_meta(), Node::Symbol(assigned) if assigned == name)))
}

/// `n = …` inside f where main has an n: a new local of f (the default, as in Python) or main's n? The fix of main's n
/// declares it where main assigns it: `global n=0`
pub(super) fn ask_local_or_global(assignment: &Node, name: &str, function: &str, main_assignment: Option<&Node>) -> Result<usize, Node> {
	use crate::diagnostic::{ask, reading, Ask, Fallback};
	let question = format!("does `{name} = …` inside {function} make a new local of {function}, or change the main-level {name}?");
	let main_level = reading(&format!("the main-level {name}"), &format!("global {name}"));
	let main_level = match main_assignment.and_then(crate::diagnostic::position) {
		Some((line, column)) => {
			let fix = crate::fixits::fix(&main_level.meaning, format!("{name} ="), format!("global {name} ="));
			main_level.fixed_by(fix.at(line, column))
		}
		None => main_level.replacing("", ""), // no main-level assignment to declare: no edit
	};
	let readings = vec![reading(&format!("a new local of {function}"), &format!("let {name} = …")).replacing(format!("{name} ="), format!("let {name} =")), main_level];
	ask(&Ask::new(LOCAL_OR_GLOBAL, question, readings, Fallback::Warning).written(&format!("{name} = …")).at_node(assignment))
}

/// `let n = …` / `var n = …` in `body`: n is explicitly the function's own
pub(super) fn declares_local(body: &Node, name: &str) -> bool {
	let mut declared = false;
	body.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			if let [keyword, declaration] = items.as_slice() {
				declared |= matches!(keyword.drop_meta(), Node::Symbol(word) if LOCAL_KEYWORDS.contains(&word.as_str()))
					&& global_binding(declaration, &Scope::new()).is_some_and(|binding| binding.name == name);
			}
		}
	});
	declared
}

/// The program's first main-level assignment of each name becomes its `global` declaration (`global n` if none)
pub(crate) fn declare_global(program: Node, names: &[String]) -> Node {
	if names.is_empty() {
		return program;
	}
	let as_global = |declaration: Node| Node::Key(Box::new(Node::Symbol("global".to_string())), Op::Colon, Box::new(declaration));
	let (mut items, bracket, separator) = match program {
		Node::List(items, bracket, separator) => (items, bracket, separator),
		single => (vec![single], Bracket::None, Separator::Newline),
	};
	for name in names {
		let assigns = |assignment: &&Node| matches!(assignment, Node::Key(target, Op::Assign, _) if declared_name(target) == Some(name));
		let found = items.iter().enumerate().find_map(|(index, item)| statement_assignment(item).filter(assigns).map(|assignment| (index, assignment.clone())));
		match found {
			Some((index, assignment)) => items[index] = as_global(assignment),
			None => items.insert(0, as_global(Node::Symbol(name.clone()))),
		}
	}
	Node::List(items, bracket, separator)
}

/// The assignment a main-level statement makes: `k = v`, and of the constant `const k = v` (as `global const k = v`)
fn statement_assignment(item: &Node) -> Option<&Node> {
	match item.drop_meta() {
		Node::List(items, _, _) => match items.as_slice() {
			[keyword, assignment] if is_constant_keyword(keyword) => Some(assignment.drop_meta()),
			_ => None,
		},
		assignment => Some(assignment),
	}
}

/// The variable a declaration binds: `k` of `k` and of the typed `k: int`
fn declared_name(target: &Node) -> Option<&String> {
	match target.drop_meta() {
		Node::Symbol(name) => Some(name),
		Node::Key(name, Op::Colon, _) => declared_name(name),
		_ => None,
	}
}

/// Is the first mention of `name` in `body` a plain `name = value` not reading it (`primes = []`)? Then the body
/// binds its own local, as in Python; reading or updating it first (`n += 1`, `xs#i = v`) means main's variable.
pub(crate) fn starts_with_fresh_binding(body: &Node, name: &str) -> bool {
	let mut first_mention = None;
	body.visit(&mut |node| {
		if first_mention.is_some() {
			return;
		}
		match node {
			Node::Key(target, Op::Assign, value) if matches!(target.drop_meta(), Node::Symbol(target) if target == name) => {
				let mut reads = false;
				value.visit(&mut |part| reads |= matches!(part, Node::Symbol(symbol) if symbol == name));
				first_mention = Some(!reads);
			}
			Node::Symbol(symbol) if symbol == name => first_mention = Some(false),
			_ => {}
		}
	});
	first_mention.unwrap_or(false)
}

/// The assignments (kept with their positions) whose changed variable satisfies `wanted`, in source order
pub(crate) fn find_assignments<'a>(node: &'a Node, wanted: &dyn Fn(&String) -> bool) -> Vec<(&'a Node, &'a String)> {
	find_changes_by(node, wanted, assignment_target_root)
}

/// The assignments and list updates (`xs.add(x)`, lowered to an assignment later) whose variable satisfies `wanted`
fn find_changes<'a>(node: &'a Node, wanted: &dyn Fn(&String) -> bool) -> Vec<(&'a Node, &'a String)> {
	find_changes_by(node, wanted, |node| assignment_target_root(node).or_else(|| updated_list_root(node)))
}

fn find_changes_by<'a>(node: &'a Node, wanted: &dyn Fn(&String) -> bool, changed: fn(&Node) -> Option<&String>) -> Vec<(&'a Node, &'a String)> {
	let mut found: Vec<(&'a Node, &'a String)> = changed(node.drop_meta()).filter(|name| wanted(name)).map(|name| (node, name)).into_iter().collect();
	match node.drop_meta() {
		Node::Key(left, _, right) => found.extend(find_changes_by(left, wanted, changed).into_iter().chain(find_changes_by(right, wanted, changed))),
		Node::List(items, _, _) => found.extend(items.iter().flat_map(|item| find_changes_by(item, wanted, changed))),
		_ => {}
	}
	found
}

/// The list variable a list method changes: `xs` in `xs.add(x)`, `xs.pop()`
fn updated_list_root(node: &Node) -> Option<&String> {
	let Node::Key(target, Op::Dot, call) = node else { return None };
	let Node::List(items, _, _) = call.drop_meta() else { return None };
	let is_update = matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(method)) if is_list_mutating_method(method));
	match target.drop_meta() {
		Node::Symbol(name) if is_update => Some(name),
		_ => None,
	}
}

/// The variable an assignment changes: `n` in `n = …`, `n += …`, `n++` and `xs#i = …`
pub(super) fn assignment_target_root(node: &Node) -> Option<&String> {
	let target = match node {
		Node::Key(target, Op::Assign | Op::Inc | Op::Dec, _) => target,
		Node::Key(target, op, _) if op.is_compound_assign() => target,
		_ => return None,
	};
	let mut target = target.drop_meta();
	// `xs#i`, `p.x`, and the declared `k: int`
	while let Node::Key(container, Op::Hash | Op::Dot | Op::Colon, _) = target {
		target = container.drop_meta();
	}
	match target {
		Node::Symbol(name) => Some(name),
		_ => None,
	}
}

/// Scope for tracking variable bindings
#[derive(Clone, Debug, Default)]
pub struct Scope {
	pub locals: HashMap<String, Local>,
	pub types: HashMap<String, Node>,  // User-defined types
	pub function_kinds: HashMap<String, Kind>,  // Return kinds of user functions, which shadow FFI names like `pow`
	/// Variables that hold a closure → `closure_new` targets they may contain (per-site closure_call kinds)
	pub closure_variable_targets: HashMap<String, HashSet<String>>,
	pub globals: HashMap<String, Local>,  // Declared `global` (kind and type, no slot): assigning them later must not create a shadowing local
	pub parent: Option<Box<Scope>>,
}

impl Scope {
	pub fn new() -> Self {
		Scope::default()
	}

	pub fn child(&self) -> Self {
		Scope {
			locals: HashMap::new(),
			types: HashMap::new(),
			function_kinds: HashMap::new(),
			closure_variable_targets: HashMap::new(),
			globals: HashMap::new(),
			parent: Some(Box::new(self.clone())),
		}
	}

	pub fn with_function_kinds(function_kinds: HashMap<String, Kind>) -> Self {
		Scope { function_kinds, ..Scope::default() }
	}

	pub fn with_closure_targets(mut self, closure_variable_targets: HashMap<String, HashSet<String>>) -> Self {
		self.closure_variable_targets = closure_variable_targets;
		self
	}

	/// Targets a closure variable may hold, looking through parent scopes
	pub fn closure_targets_of(&self, name: &str) -> Option<&HashSet<String>> {
		self.closure_variable_targets.get(name).or_else(|| self.parent.as_ref().and_then(|parent| parent.closure_targets_of(name)))
	}

	pub fn is_global(&self, name: &str) -> bool {
		self.global(name).is_some()
	}

	pub fn global(&self, name: &str) -> Option<&Local> {
		self.globals.get(name).or_else(|| self.parent.as_ref().and_then(|parent| parent.global(name)))
	}

	/// The local or declared global a name refers to, for its kind and type
	pub fn binding(&self, name: &str) -> Option<&Local> {
		self.lookup(name).or_else(|| self.global(name))
	}

	/// A local or declared global of this scope itself, to refine its type
	pub fn own_binding_mut(&mut self, name: &str) -> Option<&mut Local> {
		match self.locals.contains_key(name) {
			true => self.locals.get_mut(name),
			false => self.globals.get_mut(name),
		}
	}

	pub fn function_kind(&self, name: &str) -> Option<Kind> {
		self.function_kinds.get(name).copied().or_else(|| self.parent.as_ref().and_then(|p| p.function_kind(name)))
	}

	/// Look up a variable by name, checking parent scopes
	pub fn lookup(&self, name: &str) -> Option<&Local> {
		self.locals.get(name).or_else(||
			self.parent.as_ref().and_then(|p| p.lookup(name)))
	}

	/// The names of the variables in this scope and its parents
	pub fn local_names(&self) -> Vec<String> {
		let mut names: Vec<String> = self.locals.keys().cloned().collect();
		if let Some(parent) = &self.parent {
			names.extend(parent.local_names());
		}
		names
	}

	/// Define a new variable in current scope
	pub fn define(&mut self, name: String, type_node: Option<Box<Node>>, kind: Kind) -> Local {
		let position = self.locals.len() as u32;
		let local = Local {
			name: name.clone(),
			type_node,
			declared: false,
			position,
			is_param: false,
			kind,
			data_pointer: 0,
			data_length: 0,
		};
		self.locals.insert(name, local.clone());
		local
	}

	/// Define a function parameter
	pub fn define_param(&mut self, name: String, kind: Kind) -> Local {
		let mut local = self.define(name.clone(), None, kind);
		local.is_param = true;
		self.locals.insert(name, local.clone());
		local
	}

	/// Update a local's data pointer and length (for string assignments)
	pub fn set_local_data(&mut self, name: &str, pointer: u32, length: u32) {
		if let Some(local) = self.locals.get_mut(name) {
			local.data_pointer = pointer;
			local.data_length = length;
		}
	}

	/// Define a type in current scope
	pub fn define_type(&mut self, name: String, def: Node) {
		self.types.insert(name, def);
	}

	/// Look up a type by name
	pub fn lookup_type(&self, name: &str) -> Option<&Node> {
		self.types.get(name).or_else(||
			self.parent.as_ref().and_then(|p| p.lookup_type(name)))
	}

	/// Get total number of locals (for WASM local declaration)
	pub fn local_count(&self) -> u32 {
		self.locals.len() as u32
	}
}
