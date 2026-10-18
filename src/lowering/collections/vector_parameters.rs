//! The untyped parameters a list may reach at run time (card vector-params-cpu): `f(p) := fract(p)` called as
//! `f([0.5, 1.5])`, or a parameter its body uses as a vector (`p .* 2`, `p#2`, `p.xy`). A scalar word applied to such a
//! parameter decides at run time whether it maps over the items (broadcasting.rs `each_any`), so the same function
//! serves numbers and vectors, as its shader does; a parameter no list reaches keeps the plain call.

use super::broadcasting::element_wise_parts;
use super::nodes::word_headed;
use crate::function_values::Definition;
use crate::node::Node;
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

fn untyped_names(definition: &Definition) -> impl Iterator<Item = String> + '_ {
	definition.params.iter().filter_map(|param| param.symbol_name().map(String::from))
}

/// `name .* 2`, `2 .- name`, `name#2`, `name.xy`: the body reads name as a vector
fn used_as_vector(body: &Node, name: &str) -> bool {
	let mut found = false;
	body.visit(&mut |part| {
		let is_name = |side: &Node| side.ungrouped().is_symbol(name);
		found |= match element_wise_parts(part) {
			Some((receiver, _, operand)) => is_name(&receiver) || is_name(&operand),
			None => match part {
				Node::Key(indexed, Op::Hash, _) => is_name(indexed),
				Node::Key(vector, Op::Dot, field) => {
					is_name(vector) && field.symbol_name().and_then(crate::words::swizzle_components).is_some()
				}
				_ => false,
			},
		}
	});
	found
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
