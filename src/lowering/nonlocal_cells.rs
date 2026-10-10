//! `nonlocal y` written by a nested function (cards nonlocal-inner, nonlocal-cells): y lives in a cell outer and its
//! nested functions share, so inner's change is outer's, and a closure that escapes outer keeps that cell (each call
//! of outer makes its own). A cell holds any value (wasm_emitter/cells.rs): `y = e` → `cell_set(y·cell, e)`,
//! `y += e` → `cell_set(y·cell, cell_get(y·cell) + e)`, a read of y → `cell_get(y·cell)`. Variables only read keep the
//! captures of wasm_emitter `refresh_enclosing_captures`.

use super::nodes::{call, children_rewritten, key};
use crate::late_binding::{changes_in, declared_nonlocals, NONLOCAL};
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasm_emitter::cells::{CELL_GET, CELL_NEW, CELL_SET};

/// `y·cell`: the variable holding y's cell (no name the parser reads)
const CELL_SUFFIX: &str = "·cell";

pub fn lower(node: Node) -> Node {
	if let Some((head, op, params, body)) = definition(&node) {
		let mut body = body.clone();
		for variable in written_nonlocals(&body) {
			body = celled(body, &variable, params.contains(&variable));
		}
		return key(head.clone(), op, lower(body));
	}
	children_rewritten(node, lower)
}

/// `f(a, b) := body`: its head, operator, parameter names and body
fn definition(node: &Node) -> Option<(&Node, Op, Vec<String>, &Node)> {
	let Node::Key(head, op @ (Op::Define | Op::Assign), body) = node.drop_meta() else { return None };
	let Node::List(items, Bracket::Round, Separator::None) = head.drop_meta() else { return None };
	let (name, params) = items.split_first()?;
	matches!(name.drop_meta(), Node::Symbol(_)).then(|| (head.as_ref(), *op, params.iter().map(|param| param.drop_meta().name()).collect(), body.as_ref()))
}

/// The definitions directly in a body (not those inside them)
fn nested_definitions(node: &Node) -> Vec<&Node> {
	if definition(node).is_some() {
		return vec![node];
	}
	match node.drop_meta() {
		Node::Key(left, _, right) => [nested_definitions(left), nested_definitions(right)].concat(),
		Node::List(items, _, _) => items.iter().flat_map(nested_definitions).collect(),
		_ => vec![],
	}
}

/// The variables of this body that a nested function declares `nonlocal` and changes
fn written_nonlocals(body: &Node) -> Vec<String> {
	let mut written: Vec<String> = vec![];
	for nested in nested_definitions(body) {
		let (_, _, _, nested_body) = definition(nested).expect("a definition");
		for variable in declared_nonlocals(nested_body) {
			if !changes_in(nested_body, &variable).is_empty() && !written.contains(&variable) {
				written.push(variable);
			}
		}
	}
	written
}

/// The body with `variable` in a cell, made first (holding the parameter's value for a parameter, else ø)
fn celled(body: Node, variable: &str, is_param: bool) -> Node {
	let cell = Node::Symbol(format!("{variable}{CELL_SUFFIX}"));
	let initial = if is_param { symbol(variable) } else { Node::Empty };
	let made = key(cell.clone(), Op::Assign, call(CELL_NEW, vec![initial]));
	match through_cell(body, variable, &cell) {
		Node::List(items, bracket @ (Bracket::Curly | Bracket::None), separator @ (Separator::Semicolon | Separator::Newline)) => {
			Node::List([vec![made], items].concat(), bracket, separator)
		}
		single => Node::List(vec![made, single], Bracket::Curly, Separator::Semicolon),
	}
}

/// Reads and changes of `variable` through its cell; a nested function where it is a parameter, or its own local
/// (changed without `nonlocal`), is another variable
fn through_cell(node: Node, variable: &str, cell: &Node) -> Node {
	let is_variable = |node: &Node| matches!(node.drop_meta(), Node::Symbol(name) if name == variable);
	let get = || call(CELL_GET, vec![cell.clone()]);
	let set = |value: Node| call(CELL_SET, vec![cell.clone(), value]);
	if let Some((_, _, params, body)) = definition(&node) {
		let shadows = params.iter().any(|param| param == variable)
			|| (!changes_in(body, variable).is_empty() && !declared_nonlocals(body).iter().any(|declared| declared == variable));
		if shadows {
			return node;
		}
	}
	match node {
		Node::Symbol(ref name) if name == variable => get(),
		Node::Key(keyword, Op::Colon, names) if matches!(keyword.drop_meta(), Node::Symbol(word) if word == NONLOCAL) => Node::Key(keyword, Op::Colon, names),
		Node::Key(target, Op::Assign | Op::Define, value) if is_variable(&target) => set(through_cell(*value, variable, cell)),
		Node::Key(target, op, value) if is_variable(&target) && op.is_compound_assign() => {
			set(key(get(), op.base_op(), through_cell(*value, variable, cell)))
		}
		Node::Key(target, op @ (Op::Inc | Op::Dec), _) if is_variable(&target) => {
			set(key(get(), if op == Op::Inc { Op::Add } else { Op::Sub }, Node::from(1)))
		}
		other => children_rewritten(other, |child| through_cell(child, variable, cell)),
	}
}

/// The name of a lambda that declares `nonlocal`, made a nested function
const LAMBDA_PREFIX: &str = "lambda·";

/// A nested def changing a variable of its enclosing function declares it `nonlocal` (Python, wiki/charged.md), else
/// the change is an error naming the fix; a lambda changing one shares it without a declaration (JS, Kotlin, Swift, C#,
/// Ruby, Julia; P124). Such a lambda becomes a nested function `lambda·N` declaring the variables `nonlocal`, and its
/// reference, so it shares their cells as a nested def does. Runs before the lambda passes.
pub fn lower_lambdas(node: Node) -> Node {
	let mut count = 0;
	let node = nonlocal_lambdas(node, &mut count);
	let shared = main_lambda_changes(&node);
	crate::analyzer::declare_global(node, &shared)
}

/// The main-level variables a lambda outside any function changes (`total = 0; g(x => total += x)`): main runs once,
/// so they are shared as globals, which closures never copy (P124)
fn main_lambda_changes(program: &Node) -> Vec<String> {
	let mut locals = vec![];
	collect_locals(program, &mut locals);
	let mut shared: Vec<String> = vec![];
	main_lambdas(program, &mut |params, body| {
		for (_, name) in undeclared_changes(body, &params, &locals) {
			if !shared.contains(&name) {
				shared.push(name);
			}
		}
	});
	shared
}

/// The lambdas outside function definitions, with their parameter names and bodies
fn main_lambdas(node: &Node, action: &mut impl FnMut(Vec<String>, &Node)) {
	if definition(node).is_some() {
		return;
	}
	match node.drop_meta() {
		Node::Key(params, Op::FatArrow, body) => action(lambda_parameters(params), body),
		Node::Key(left, _, right) => {
			main_lambdas(left, action);
			main_lambdas(right, action);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| main_lambdas(item, action)),
		_ => {}
	}
}

fn nonlocal_lambdas(node: Node, count: &mut usize) -> Node {
	if let Some((head, op, params, body)) = definition(&node) {
		let mut locals = params;
		collect_locals(body, &mut locals);
		let mut hoisted = vec![];
		let body = in_enclosing(body.clone(), &locals, count, &mut hoisted);
		return key(head.clone(), op, with_first(hoisted, body)).with_meta_of(&node);
	}
	node.map_children(|child| nonlocal_lambdas(child, count))
}

/// The body with the `statements` before its own
fn with_first(statements: Vec<Node>, body: Node) -> Node {
	if statements.is_empty() {
		return body;
	}
	match body {
		Node::List(items, Bracket::Curly, Separator::Semicolon | Separator::Newline) => Node::List([statements, items].concat(), Bracket::Curly, Separator::Semicolon),
		single => Node::List([statements, vec![single]].concat(), Bracket::Curly, Separator::Semicolon),
	}
}

/// The body of a function whose `locals` its nested functions and lambdas may change only when declared `nonlocal`;
/// a lambda declaring it is `hoisted` as a nested function
fn in_enclosing(node: Node, locals: &[String], count: &mut usize, hoisted: &mut Vec<Node>) -> Node {
	if let Some((_, _, params, body)) = definition(&node) {
		if let Some(error) = undeclared_change_error(body, &params, locals) {
			return error;
		}
		return nonlocal_lambdas(node, count);
	}
	match node {
		Node::Key(lambda_params, Op::FatArrow, body) => {
			let params = lambda_parameters(&lambda_params);
			// a lambda changing an enclosing local shares it, as in JS, Kotlin, Swift, C#, Ruby and Julia
			let shared: Vec<Node> = undeclared_changes(&body, &params, locals).into_iter()
				.map(|(_, name)| key(symbol(NONLOCAL), Op::Colon, Node::Symbol(name))).collect();
			let body = nonlocal_lambdas(with_first(shared, *body), count);
			if declared_nonlocals(&body).is_empty() {
				return Node::Key(lambda_params, Op::FatArrow, Box::new(body));
			}
			*count += 1;
			let name = format!("{LAMBDA_PREFIX}{count}");
			let head = call(&name.clone(), params.into_iter().map(Node::Symbol).collect());
			hoisted.push(key(head, Op::Define, body));
			crate::closures::function_reference(name)
		}
		other => other.map_children(|child| in_enclosing(child, locals, count, hoisted)),
	}
}

/// `x`, `(a, b)`, `ø`: the parameter names of a lambda
fn lambda_parameters(params: &Node) -> Vec<String> {
	match params.drop_meta() {
		Node::Symbol(name) => vec![name.clone()],
		Node::List(items, _, _) => items.iter().map(|item| item.drop_meta().name()).collect(),
		_ => vec![],
	}
}

/// The variables a body assigns, not inside its nested functions or lambdas
fn collect_locals(node: &Node, locals: &mut Vec<String>) {
	if definition(node).is_some() {
		return;
	}
	match node.drop_meta() {
		Node::Key(_, Op::FatArrow, _) => {}
		Node::Key(target, op, value) if *op == Op::Assign || op.is_compound_assign() => {
			if let Node::Symbol(name) = target.drop_meta() {
				if !locals.contains(name) {
					locals.push(name.clone());
				}
			}
			collect_locals(value, locals);
		}
		Node::Key(left, _, right) => {
			collect_locals(left, locals);
			collect_locals(right, locals);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| collect_locals(item, locals)),
		_ => {}
	}
}

/// The changes of enclosing locals in a nested body that neither declares them `nonlocal` nor has them as parameters:
/// `n += 1`, `n++`, `n = n + 1` (a fresh `n = 5` is a local of its own), with the variable changed
fn undeclared_changes<'a>(body: &'a Node, params: &[String], locals: &[String]) -> Vec<(&'a Node, String)> {
	let declared = declared_nonlocals(body);
	let mut found: Vec<(&Node, String)> = vec![];
	body.visit(&mut |part| {
		let Node::Key(target, op, value) = part else { return };
		let Node::Symbol(name) = target.drop_meta() else { return };
		let reads_itself = *op == Op::Assign && { let mut reads = false; value.visit(&mut |read| reads |= matches!(read, Node::Symbol(read) if read == name)); reads };
		let updates = op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec) || reads_itself;
		if updates && locals.contains(name) && !params.contains(name) && !declared.contains(name) && !found.iter().any(|(_, seen)| seen == name) {
			found.push((part, name.clone()));
		}
	});
	found
}

/// A nested def changing an enclosing local without `nonlocal` (Python's rule for def)
fn undeclared_change_error(body: &Node, params: &[String], locals: &[String]) -> Option<Node> {
	let (node, name) = undeclared_changes(body, params, locals).into_iter().next()?;
	Some(crate::diagnostic::Diagnostic::at(node, format!(
		"{name} belongs to the enclosing function: declare `nonlocal {name}` to change it from this inner function, or use a new local name"))
		.fix(format!("nonlocal {name}")).into_error())
}
