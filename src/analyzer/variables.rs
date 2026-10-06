//! Variables and scope: collecting a program's variables, main-level bindings, the Scope of locals

use super::*;

/// Collect variables defined in node and populate scope
/// Returns count of temp locals needed (e.g., for while loops)
pub fn collect_variables(node: &Node, scope: &mut Scope) -> u32 {
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
	(kind, (kind == Kind::List).then(|| Box::new(Node::Symbol(list_type_name(value, scope)))))
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
					// But still count any variables in the value expression
					if let Some(global) = global_binding(right, scope).filter(|global| !scope.is_global(&global.name)) {
						scope.globals.insert(global.name.clone(), global);
					}
					return collect_variables_inner(right, scope, true, false);
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
						scope.define(name.clone(), type_node, kind);
					}
					Node::Symbol(name) => {
						type_list_by_first_append(name, right, scope);
						widen_to_float(scope, name, right);
						widen_to_node(scope, name, right);
					}
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
	if let Some(local) = scope.locals.get_mut(name).filter(|local| local.kind == Kind::Empty && local.type_node.is_none()) {
		local.kind = Kind::List;
		local.type_node = Some(Box::new(Node::Symbol(appended_type)));
	}
}

/// An exact variable that is later assigned an f64 (`x=10; x=floor(2.5)` with libm's floor) holds an f64 throughout,
/// as an expression mixing in an f64 is one; a declared type is kept (and checked elsewhere)
pub(super) fn widen_to_float(scope: &mut Scope, name: &str, value: &Node) {
	let float_value = infer_type(value, scope).is_float();
	if let Some(local) = scope.locals.get_mut(name).filter(|local| local.kind == Kind::Int && local.type_node.is_none()) {
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
	if CONCRETE_KINDS.contains(&local.kind) && CONCRETE_KINDS.contains(&assigned) && !mixes(local.kind, assigned) {
		local.kind = Kind::Empty;
		local.type_node = None;
	}
}

/// The kind a value evidently has as written: a literal, a list literal, or arithmetic of numbers; None for anything else
/// (a variable, a call, a loop variable), whose kind only inference knows
pub(super) fn evident_kind(value: &Node) -> Option<Kind> {
	match value.drop_meta() {
		Node::Number(Number::Float(_)) => Some(Kind::Float),
		Node::Number(_) => Some(Kind::Int),
		Node::Text(_) | Node::Char(_) => Some(Kind::Text),
		Node::List(_, Bracket::Square, _) => Some(Kind::List),
		Node::Key(left, op, right) if op.is_arithmetic() => match (evident_kind(left)?, evident_kind(right)?) {
			(Kind::Int, Kind::Int) => Some(Kind::Int),
			(Kind::Int | Kind::Float, Kind::Int | Kind::Float) => Some(Kind::Float),
			_ => None,
		},
		_ => None,
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
	bodies.into_iter().find_map(|body| first_kind_change(body, &mut HashMap::new()))
}

/// Walks the assignments in program order, not into function definitions or lambdas (their own scopes);
/// `kinds` holds each variable's evident kind, None once it was given a value of no evident kind
pub(super) fn first_kind_change(node: &Node, kinds: &mut HashMap<String, Option<Kind>>) -> Option<Diagnostic> {
	let mixes = |a: Kind, b: Kind| a == b || [a, b].iter().all(|kind| matches!(kind, Kind::Int | Kind::Float));
	match node.drop_meta() {
		Node::Key(_, Op::Define | Op::Arrow | Op::FatArrow, _) => None,
		Node::Key(target, Op::Assign, value) => {
			if let Some(change) = first_kind_change(value, kinds) {
				return Some(change);
			}
			let Node::Symbol(name) = target.drop_meta() else { return None };
			let (earlier, given) = (kinds.get(name).copied(), evident_kind(value));
			if let (Some(Some(was)), Some(now)) = (earlier, given) {
				if !mixes(was, now) {
					let message = format!("{name} was {}, is given {}: use another name", kind_with_article(was), kind_with_article(now));
					return Some(Diagnostic::at(value, message));
				}
			}
			// a first evident value fixes the kind (Int widens to Float); any value of no evident kind ends the check
			let kind = match (earlier, given) {
				(None, given) => given,
				(Some(Some(was)), Some(now)) => Some(if now == Kind::Float { now } else { was }),
				_ => None,
			};
			kinds.insert(name.clone(), kind);
			None
		}
		Node::Key(left, _, right) => first_kind_change(left, kinds).or_else(|| first_kind_change(right, kinds)),
		Node::List(items, _, _) => items.iter().find_map(|item| first_kind_change(item, kinds)),
		_ => None,
	}
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
	for function in functions {
		let is_main_variable = |name: &String| main.lookup(name).is_some() && !main.is_global(name)
			&& !function.params.iter().any(|param| param.name == *name) && !declares_local(&function.body, name);
		let mut decided: HashSet<&String> = HashSet::new();
		for (node, name) in find_assignments(&function.body, &is_main_variable) {
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
		let assigns = |item: &Node| matches!(item.drop_meta(), Node::Key(target, Op::Assign, _) if matches!(target.drop_meta(), Node::Symbol(target) if target == name));
		match items.iter().position(assigns) {
			Some(index) => items[index] = as_global(items[index].clone()),
			None => items.insert(0, as_global(Node::Symbol(name.clone()))),
		}
	}
	Node::List(items, bracket, separator)
}

/// Is the first mention of `name` in `body` a plain `name = value` not reading it (`primes = []`)? Then the body
/// binds its own local, as in Python; reading or updating it first (`n += 1`, `xs#i = v`) means main's variable.
pub(super) fn starts_with_fresh_binding(body: &Node, name: &str) -> bool {
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
	let mut found: Vec<(&'a Node, &'a String)> = assignment_target_root(node.drop_meta()).filter(|name| wanted(name)).map(|name| (node, name)).into_iter().collect();
	match node.drop_meta() {
		Node::Key(left, _, right) => found.extend(find_assignments(left, wanted).into_iter().chain(find_assignments(right, wanted))),
		Node::List(items, _, _) => found.extend(items.iter().flat_map(|item| find_assignments(item, wanted))),
		_ => {}
	}
	found
}

/// The variable an assignment changes: `n` in `n = …`, `n += …`, `n++` and `xs#i = …`
pub(super) fn assignment_target_root(node: &Node) -> Option<&String> {
	let target = match node {
		Node::Key(target, Op::Assign | Op::Inc | Op::Dec, _) => target,
		Node::Key(target, op, _) if op.is_compound_assign() => target,
		_ => return None,
	};
	let mut target = target.drop_meta();
	while let Node::Key(container, Op::Hash | Op::Dot, _) = target {
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
