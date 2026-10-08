//! `use list` in the type model: the standard modules are written in warp (lib/*.warp, notes/stdlib.md), so `use X`
//! becomes the definitions of X the program calls, and the ones those call, as if the program had written them.

use super::{function_definition, is_word, mentions, statements};
use crate::modules::{std_module_source, USE_KEYWORDS};
use crate::node::{Bracket, Node, Separator};

/// The program with each `use` of a standard module replaced by the definitions it needs from it
pub(super) fn inline(program: &Node) -> Result<Node, String> {
	let parts = statements(program);
	if !parts.iter().any(|statement| used_module(statement).is_some()) {
		return Ok(program.clone());
	}
	let own: Vec<&Node> = parts.iter().copied().filter(|statement| used_module(statement).is_none()).collect();
	let mut inlined = vec![];
	for statement in &parts {
		match used_module(statement) {
			Some(module) => {
				let source = std_module_source(&module).ok_or_else(|| format!("not in W0: use {module}"))?;
				inlined.extend(needed_definitions(&crate::warp_parser::parse(source), &own));
			}
			None => inlined.push((*statement).clone()),
		}
	}
	Ok(Node::List(inlined, Bracket::None, Separator::Semicolon))
}

/// `use list`: the module's name
fn used_module(statement: &Node) -> Option<String> {
	match statement.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 && USE_KEYWORDS.iter().any(|keyword| is_word(&items[0], keyword)) => match items[1].drop_meta() {
			Node::Symbol(name) | Node::Text(name) => Some(name.clone()),
			_ => None,
		},
		_ => None,
	}
}

/// The module's definitions the program calls, transitively, in the module's order; not the ones it defines itself
fn needed_definitions(module: &Node, program: &[&Node]) -> Vec<Node> {
	let definitions: Vec<(&str, &Node)> = statements(module).into_iter().filter_map(|statement| function_definition(statement).map(|(name, _, _)| (name, statement))).collect();
	let defined_by_program: Vec<&str> = program.iter().filter_map(|statement| function_definition(statement).map(|(name, _, _)| name)).collect();
	let mut needed: Vec<&str> = vec![];
	let mut callers: Vec<&Node> = program.to_vec();
	while let Some(caller) = callers.pop() {
		for (name, definition) in &definitions {
			if !needed.contains(name) && !defined_by_program.contains(name) && mentions(caller, name) {
				needed.push(name);
				callers.push(definition);
			}
		}
	}
	definitions.into_iter().filter(|(name, _)| needed.contains(name)).map(|(_, definition)| definition.clone()).collect()
}
