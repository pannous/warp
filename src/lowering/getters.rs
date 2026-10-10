//! P71 (user, 2026-10-06): `name := expr` without parameters is always charged (wiki/charged.md §2): a getter that
//! evaluates expr at every use. The definition becomes `name() := expr` and every later read of `name` the call
//! `name()`, so each emitter path that handles calls handles getters; late_binding lets a getter read the current
//! value of a variable changed after it. Assigning a getter afterwards (`z = 6`, `z += 1`) is an error naming the
//! definition. Not getters: `name := {…}` (a block or a list), `name := x => …` and `name := …it…` (functions), and a
//! name applied to arguments later (`sum := fold +; sum [1 2 3]`, a function value).

use super::nodes::{call, key};
use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

/// marks the `name()` that `name := e` became: written without parentheses, read as a value (late_binding warns of effects)
const BARE_GETTER_MARK: &str = "getter·bare";

pub fn lower(program: Node) -> Node {
	lower_in(program, &[])
}

/// A getter in scope: its name and its definition as written (`z := y*y`), for the error of an assignment
#[derive(Clone)]
struct Getter {
	name: String,
	written: String,
}

/// `name := expr` where expr is not in braces, no lambda and reads no `it` / `$0`: a getter; its name
pub(crate) fn getter_name(statement: &Node) -> Option<&str> {
	let Node::Key(left, Op::Define, body) = statement.drop_meta() else { return None };
	let Node::Symbol(name) = left.drop_meta() else { return None };
	let function_like = matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _) | Node::Key(_, Op::FatArrow, _));
	(!function_like && !crate::analyzer::takes_implicit_parameter(body)).then_some(name.as_str())
}

fn lower_in(node: Node, active: &[Getter]) -> Node {
	if active.is_empty() && !mentions_define(&node) {
		return node;
	}
	let shadowed = shadowed_names(&node);
	let narrowed: Vec<Getter>;
	let active = if shadowed.is_empty() { active } else {
		narrowed = active.iter().filter(|getter| !shadowed.contains(&getter.name)).cloned().collect();
		&narrowed
	};
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower_in(*node, active)), data },
		Node::Symbol(name) if is_active(active, &name) => call(&name, vec![]),
		Node::List(items, bracket @ (Bracket::None | Bracket::Curly), separator @ (Separator::Semicolon | Separator::Newline)) => {
			Node::List(lower_statements(items, active), bracket, separator)
		}
		Node::Key(left, op, right) if is_binding(&op) && matches!(left.drop_meta(), Node::Symbol(name) if is_active(active, name)) => {
			let getter = active.iter().rev().find(|getter| left.drop_meta().name() == getter.name).expect("guarded");
			assignment_error(&Node::Key(left.clone(), op, right), getter)
		}
		Node::Key(left, Op::Colon, right) if matches!(left.drop_meta(), Node::Symbol(_)) => Node::Key(left, Op::Colon, Box::new(lower_in(*right, active))),
		// `xs.add([b, a])`: the method's arguments read getters, its name does not
		Node::Key(left, Op::Dot, right) => {
			let right = match right.drop_meta().clone() {
				Node::List(items, bracket, separator) if items.len() > 1 => {
					let mut items = items.into_iter();
					let method = items.next().expect("a method");
					Node::List(std::iter::once(method).chain(items.map(|item| lower_in(item, active))).collect(), bracket, separator)
				}
				_ => *right,
			};
			key(lower_in(*left, active), Op::Dot, right)
		}
		Node::Key(left, op, right) => key(lower_in(*left, active), op, lower_in(*right, active)),
		Node::List(items, Bracket::Round, separator) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(head)) if is_active(active, head)) => {
			let mut items = items.into_iter();
			let head = items.next().expect("a head");
			Node::List(std::iter::once(head).chain(items.map(|item| lower_in(item, active))).collect(), Bracket::Round, separator)
		}
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| lower_in(item, active)).collect(), bracket, separator),
		other => other,
	}
}

/// The statements of one list: a getter definition is visible to the statements after it
fn lower_statements(items: Vec<Node>, active: &[Getter]) -> Vec<Node> {
	let mut active = active.to_vec();
	let mut lowered = Vec::with_capacity(items.len());
	for index in 0..items.len() {
		let item = &items[index];
		match getter_name(item).filter(|name| !applied_in(&items[index + 1..], name)).map(str::to_string) {
			Some(name) => {
				lowered.push(lower_in(getter_definition(item.clone()), &active));
				active.retain(|getter| getter.name != name);
				active.push(Getter { written: written_definition(item), name });
			}
			None => lowered.push(lower_in(item.clone(), &active)),
		}
	}
	lowered
}

fn is_active(active: &[Getter], name: &str) -> bool {
	active.iter().any(|getter| getter.name == name)
}

fn is_binding(op: &Op) -> bool {
	matches!(op, Op::Assign | Op::Define) || op.is_compound_assign()
}

fn mentions_define(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Key(_, Op::Define, _)));
	found
}

/// The names a lambda or a function definition binds itself: its parameters, the variables its body assigns and its loop
/// variables (locals, wiki/charged.md §3)
pub(crate) fn shadowed_names(node: &Node) -> Vec<String> {
	match node.drop_meta() {
		Node::Key(parameters, Op::FatArrow, _) => {
			let mut names = vec![];
			parameters.visit(&mut |part| if let Node::Symbol(name) = part { names.push(name.clone()) });
			names
		}
		Node::Key(_, Op::Define | Op::Assign, _) => locals_of(node),
		Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if crate::operators::is_function_keyword(word)) => locals_of(node),
		_ => vec![],
	}
}

fn locals_of(definition: &Node) -> Vec<String> {
	crate::late_binding::functions_in(definition).into_iter()
		.flat_map(|function| {
			let assigned: Vec<String> = crate::analyzer::find_assignments(&function.body, &|_| true).into_iter().map(|(_, name)| name.clone()).collect();
			let looped = crate::lowering::for_loop::loop_variables(&function.body);
			function.params.into_iter().map(|param| param.name).chain(assigned).chain(looped).collect::<Vec<_>>()
		})
		.collect()
}

/// Is `name` called with arguments in the statements: `name [1 2 3]`, `name 1 2`, `name(x)`
fn applied_in(statements: &[Node], name: &str) -> bool {
	let mut found = false;
	for statement in statements {
		statement.visit(&mut |part| found |= matches!(part, Node::List(items, _, Separator::Space | Separator::None)
			if items.len() > 1 && matches!(items[0].drop_meta(), Node::Symbol(head) if head == name)));
	}
	found
}

/// `z := e` → `z() := e`, the `z()` marked as written bare
fn getter_definition(statement: Node) -> Node {
	match statement {
		Node::Meta { node, data } => Node::Meta { node: Box::new(getter_definition(*node)), data },
		Node::Key(left, Op::Define, body) => match left.drop_meta() {
			Node::Symbol(name) => Node::Key(Box::new(Node::meta(call(name, vec![]), Node::Symbol(BARE_GETTER_MARK.to_string()))), Op::Define, body),
			_ => Node::Key(left, Op::Define, body),
		},
		other => other,
	}
}

/// `z()` of a getter written `z := e`, not `z() := e`
pub(crate) fn is_written_bare(left: &Node) -> bool {
	match left {
		Node::Meta { node, data } => matches!(data.drop_meta(), Node::Symbol(mark) if mark == BARE_GETTER_MARK) || is_written_bare(node),
		_ => false,
	}
}

fn written_definition(statement: &Node) -> String {
	match statement.drop_meta() {
		Node::Key(left, _, body) => format!("{} := {}", left.drop_meta().name(), body.drop_meta().serialize().trim()),
		other => other.serialize().trim().to_string(),
	}
}

/// `z = 6` after `z := y*y`: z runs its body at every use, it holds no value to replace
fn assignment_error(assignment: &Node, getter: &Getter) -> Node {
	let name = &getter.name;
	Diagnostic::at(assignment, format!("{name} is charged ({}): it runs at every use and cannot be assigned; write {name} = … at the definition for a value, or use another name", getter.written))
		.into_error()
}
