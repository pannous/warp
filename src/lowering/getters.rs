//! P71 (user, 2026-10-06): `name := expr` without parameters is always charged (wiki/charged.md §2): a getter that
//! evaluates expr at every use. The definition becomes `name() := expr` and every later read of `name` the call
//! `name()`, so each emitter path that handles calls handles getters; late_binding lets a getter read the current
//! value of a variable changed after it. Assigning a getter afterwards (`z = 6`) is an error naming the definition.
//! Not here: `name := {statements}` (already a function) and `name := …it…` (a function of `it`).

use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

pub fn lower(program: Node) -> Node {
	lower_statement_lists(program, &[])
}

/// `name := expr` where expr is no statement block and reads no `it` / `$0`: a getter; its name
pub(crate) fn getter_name(statement: &Node) -> Option<&str> {
	let Node::Key(left, Op::Define, body) = statement.drop_meta() else { return None };
	let Node::Symbol(name) = left.drop_meta() else { return None };
	(!crate::analyzer::is_statement_block(body) && !crate::analyzer::takes_implicit_parameter(body)).then_some(name.as_str())
}

/// Every statement list of `node`, with the getters `active` from enclosing lists: definitions rewritten, reads called
fn lower_statement_lists(node: Node, active: &[String]) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower_statement_lists(*node, active)), data },
		Node::List(items, bracket, separator @ (Separator::Semicolon | Separator::Newline)) if matches!(bracket, Bracket::None | Bracket::Curly) => {
			let mut active = visible_in(&items, active);
			let mut lowered = Vec::with_capacity(items.len());
			for item in items {
				if let Some(name) = getter_name(&item).map(str::to_string) {
					lowered.push(lower_statement_lists(getter_definition(item), &active));
					active.push(name);
				} else if let Some(name) = active.iter().find(|name| assigns(&item, name)) {
					lowered.push(assignment_error(&item, name, &lowered));
				} else {
					lowered.push(lower_statement_lists(called_reads(item, &active), &active));
				}
			}
			Node::List(lowered, bracket, separator)
		}
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| lower_statement_lists(item, active)).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(lower_statement_lists(*left, active)), op, Box::new(lower_statement_lists(*right, active))),
		other => other,
	}
}

/// The getters of enclosing lists that this list neither defines anew nor binds as a variable
fn visible_in(statements: &[Node], active: &[String]) -> Vec<String> {
	active.iter().filter(|name| !statements.iter().any(|statement| binds(statement, name))).cloned().collect()
}

/// `z = …`, `z := …`, `z += …`: a statement binding or changing the name
fn binds(statement: &Node, name: &str) -> bool {
	matches!(statement.drop_meta(), Node::Key(left, op, _) if (matches!(op, Op::Assign | Op::Define) || op.is_compound_assign()) && is_name(left, name))
}

fn assigns(statement: &Node, name: &str) -> bool {
	binds(statement, name) && getter_name(statement) != Some(name)
}

fn is_name(node: &Node, name: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == name)
}

fn call(name: &str) -> Node {
	Node::List(vec![Node::Symbol(name.to_string())], Bracket::Round, Separator::None)
}

/// `z := e` → `z() := e`
fn getter_definition(statement: Node) -> Node {
	match statement {
		Node::Meta { node, data } => Node::Meta { node: Box::new(getter_definition(*node)), data },
		Node::Key(left, Op::Define, body) => match left.drop_meta() {
			Node::Symbol(name) => Node::Key(Box::new(call(name)), Op::Define, body),
			_ => Node::Key(left, Op::Define, body),
		},
		other => other,
	}
}

/// The reads of the getters in `node` as calls; a key, a member name, an explicit call and a parameter of the same
/// name stay as written
fn called_reads(node: Node, getters: &[String]) -> Node {
	if getters.is_empty() {
		return node;
	}
	let getters: Vec<String> = getters.iter().filter(|name| !has_parameter(&node, name)).cloned().collect();
	match node {
		Node::Symbol(name) if getters.contains(&name) => call(&name),
		Node::Meta { node, data } => Node::Meta { node: Box::new(called_reads(*node, &getters)), data },
		Node::Key(left, Op::Colon, right) if matches!(left.drop_meta(), Node::Symbol(_)) => Node::Key(left, Op::Colon, Box::new(called_reads(*right, &getters))),
		Node::Key(left, Op::Dot, right) => Node::Key(Box::new(called_reads(*left, &getters)), Op::Dot, right),
		Node::Key(left, op, right) => Node::Key(Box::new(called_reads(*left, &getters)), op, Box::new(called_reads(*right, &getters))),
		Node::List(items, Bracket::Round, separator) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(head)) if getters.contains(head)) => {
			let mut items = items.into_iter();
			let head = items.next().expect("a head");
			Node::List(std::iter::once(head).chain(items.map(|item| called_reads(item, &getters))).collect(), Bracket::Round, separator)
		}
		// a nested statement list is lowered on its own (lower_statement_lists), which sees its own bindings
		Node::List(items, bracket, separator @ (Separator::Semicolon | Separator::Newline)) if matches!(bracket, Bracket::None | Bracket::Curly) => Node::List(items, bracket, separator),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| called_reads(item, &getters)).collect(), bracket, separator),
		other => other,
	}
}

/// A lambda `z => …` or a function definition with the parameter `z`: inside it z is the parameter
fn has_parameter(node: &Node, name: &str) -> bool {
	match node.drop_meta() {
		Node::Key(parameters, Op::FatArrow, _) => {
			let mut found = false;
			parameters.visit(&mut |part| found |= is_name(part, name));
			found
		}
		_ => crate::late_binding::functions_in(node).iter().any(|function| function.params.iter().any(|param| param.name == name)),
	}
}

/// `z = 6` after `z := y*y`: z runs its body at every use, it holds no value to replace
fn assignment_error(statement: &Node, name: &str, before: &[Node]) -> Node {
	let definition = before.iter().rev().find(|earlier| defines_getter(earlier, name)).map(written_definition).unwrap_or_else(|| format!("{name} := …"));
	Diagnostic::at(statement, format!("{name} is charged ({definition}): it runs at every use and cannot be assigned; write {name} = … at the definition for a value, or use another name"))
		.into_error()
}

fn defines_getter(statement: &Node, name: &str) -> bool {
	matches!(statement.drop_meta(), Node::Key(left, Op::Define, _) if matches!(left.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() == 1 && is_name(&items[0], name)))
}

fn written_definition(statement: &Node) -> String {
	let Node::Key(left, _, body) = statement.drop_meta() else { return String::new() };
	let name = left.drop_meta().serialize().trim().trim_start_matches('(').trim_end_matches(')').to_string();
	format!("{name} := {}", body.drop_meta().serialize().trim())
}
