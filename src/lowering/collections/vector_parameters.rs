//! The untyped parameters a list may reach at run time (card vector-params-cpu): `f(p) := fract(p)` called as
//! `f([0.5, 1.5])`, or a parameter its body uses as a vector (`p .* 2`, `p#2`, `p.xy`). A scalar word applied to such a
//! parameter decides at run time whether it maps over the items (broadcasting.rs `each_any`), so the same function
//! serves numbers and vectors, as its shader does; a parameter no list reaches keeps the plain call.

use super::broadcasting::element_wise_parts;
use super::nodes::{any_part, word_headed};
use crate::function_values::Definition;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

/// Per function name, its parameters a list may reach: passed one where the program calls it (also through another
/// such parameter), or used as a vector in its body; `broadcasting` functions map over a list before the call
pub(crate) fn vector_parameters(
	definitions: &[Definition],
	program: &Node,
	is_list: impl Fn(&Node) -> bool,
	broadcasting: &HashSet<String>,
) -> HashMap<String, HashSet<String>> {
	let mut vectors: HashMap<String, HashSet<String>> = HashMap::new();
	for definition in definitions {
		let used_as_vectors = untyped_names(definition).filter(|name| used_as_vector(&definition.body, name));
		vectors.entry(definition.name.clone()).or_default().extend(used_as_vectors);
	}
	loop {
		let mut passed = lists_passed(program, definitions, &|argument| is_list(argument), broadcasting);
		for definition in definitions {
			let inside = vectors.get(&definition.name).cloned().unwrap_or_default();
			let reaching = |argument: &Node| {
				is_list(argument) || argument.symbol_name().is_some_and(|name| inside.contains(name))
			};
			passed.extend(lists_passed(&definition.body, definitions, &reaching, broadcasting));
		}
		let mut changed = false;
		for (function, parameter) in passed {
			changed |= vectors.entry(function).or_default().insert(parameter);
		}
		if !changed {
			return vectors;
		}
	}
}

/// The functions whose result is a list: a list literal, an element-wise expression or the call of another such
/// function (`agx_display(c) := { …; [dot(look, r), dot(look, g), dot(look, b)] }`)
pub(crate) fn list_functions(definitions: &[Definition]) -> HashSet<String> {
	let mut found = HashSet::new();
	loop {
		let before = found.len();
		for definition in definitions {
			let result = crate::analyzer::block_result(&definition.body).unwrap_or_else(|| definition.body.clone());
			if is_list_result(&result, &found) {
				found.insert(definition.name.clone());
			}
		}
		if found.len() == before {
			return found;
		}
	}
}

/// `[a, b]`, `a .* b`, `f(x)` of a function in `lists`
pub(crate) fn is_list_result(node: &Node, lists: &HashSet<String>) -> bool {
	let node = node.ungrouped();
	matches!(node, Node::List(items, Bracket::Square, _) if !items.is_empty())
		|| element_wise_parts(node).is_some()
		|| matches!(node, Node::List(items, _, _) if word_headed(items).is_some_and(|(name, _)| lists.contains(name)))
}

fn untyped_names(definition: &Definition) -> impl Iterator<Item = String> + '_ {
	definition.params.iter().filter_map(|param| param.symbol_name().map(String::from))
}

/// `name .* 2`, `2 .- name`, `name#2`, `name.xy`: the body reads name as a vector
fn used_as_vector(body: &Node, name: &str) -> bool {
	let element_wise_operand = |part: &Node| {
		element_wise_parts(part).is_some_and(|(receiver, _, operand)| [receiver, operand].iter().any(|side| side.ungrouped().is_symbol(name)))
	};
	reads_components(body, name) || any_part(body, element_wise_operand)
}

/// `name#2`, `name.x`, `name.xy`: the body reads components of name, a vector, never one number; also of a variable
/// computed element-wise from it (`inside = p .- cell; blend = inside .* inside; blend.x`)
pub(crate) fn reads_components(body: &Node, name: &str) -> bool {
	let vectors = element_wise_from(body, name);
	let is_vector = |node: &Node| node.ungrouped().symbol_name().is_some_and(|variable| vectors.contains(variable));
	any_part(body, |part| match part {
		Node::Key(indexed, Op::Hash, _) => is_vector(indexed),
		Node::Key(vector, Op::Dot, field) => {
			is_vector(vector) && field.symbol_name().and_then(crate::words::swizzle_components).is_some()
		}
		_ => false,
	})
}

/// name and the variables of body assigned an element-wise expression of one of them
fn element_wise_from(body: &Node, name: &str) -> HashSet<String> {
	let mut found = HashSet::from([name.to_string()]);
	loop {
		let before = found.len();
		body.visit(&mut |part| {
			let Node::Key(target, Op::Assign | Op::Define, value) = part else { return };
			let derived = element_wise_parts(value).is_some() && value.mentions_where(|word| found.contains(word));
			if let Some(variable) = target.symbol_name().filter(|_| derived) {
				found.insert(variable.to_string());
			}
		});
		if found.len() == before {
			return found;
		}
	}
}

/// The (function, parameter) pairs that the calls in `node` pass a list `reaching` says is one
fn lists_passed(
	node: &Node,
	definitions: &[Definition],
	reaching: &dyn Fn(&Node) -> bool,
	broadcasting: &HashSet<String>,
) -> Vec<(String, String)> {
	let mut passed = Vec::new();
	node.visit(&mut |part| {
		let Node::List(items, _, _) = part else { return };
		let Some((name, arguments)) = word_headed(items) else { return };
		if broadcasting.contains(name) {
			return;
		}
		let called = definitions.iter().filter(|definition| definition.name == name);
		for definition in called.filter(|definition| definition.params.len() == arguments.len()) {
			for (param, argument) in definition.params.iter().zip(arguments) {
				if let Some(parameter) = param.symbol_name().filter(|_| reaching(argument)) {
					passed.push((name.to_string(), parameter.to_string()));
				}
			}
		}
	});
	passed
}
