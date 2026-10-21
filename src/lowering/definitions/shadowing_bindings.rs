//! A local named like a function of the program is the local wherever the function binds it (cards local-named-like,
//! the family of binder-locals and program-var-leaks): `f() := { arity = [3]; arity in [[3]] }` tests the list,
//! `items[1..]` slices it, `g [1, 2]` and `g + 1` read it, whatever `arity(s)`, `items(p, closer)` or `g(x)` the
//! program defines. Each such local is renamed apart (`arity` → `arity·local`) before any pass reads a word as a
//! function's call. A call `g(3)` of a local that holds no function value is an error naming both.

use crate::errors::diagnostic::Diagnostic;
use crate::modules::needed_definitions::renamed_words;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::program_names::{bound_inside, ProgramNames};
use crate::type_tests::function_definition;
use std::collections::HashMap;

const LOCAL_SUFFIX: &str = "·local";

pub fn lower(program: Node) -> Node {
	let names = ProgramNames::of(&program);
	apart(program, &names)
}

fn apart(node: Node, names: &ProgramNames) -> Node {
	let node = match local_renames(&node, names) {
		Some(renames) => {
			let renamed = renamed_words(node, &renames);
			match plain_value_call(&renamed, &renames) {
				Some(error) => return error,
				None => renamed,
			}
		}
		None => node,
	};
	node.map_children(|child| apart(child, names))
}

/// The locals of the function `node` defines that share a name with one of the program's functions, each mapped to
/// its name apart; None for anything but such a function
fn local_renames(node: &Node, names: &ProgramNames) -> Option<HashMap<String, String>> {
	let own_name = crate::lowering::function_head(node)?.name.to_string();
	let renames: HashMap<String, String> = bound_inside(node)?
		.into_iter()
		.filter(|local| *local != own_name && names.function_takes(local, 0).is_some())
		.map(|local| (local.clone(), format!("{local}{LOCAL_SUFFIX}")))
		.collect();
	(!renames.is_empty()).then_some(renames)
}

/// `g(3)` in a function whose local g holds only plain values (`g = 5`): the error naming the local and the function
fn plain_value_call(definition: &Node, renames: &HashMap<String, String>) -> Option<Node> {
	let (_, body, _) = function_definition(definition)?;
	renames.iter().find_map(|(written, local)| {
		let values = assigned_values(&body, local);
		let plain = values.first().filter(|_| values.iter().all(|value| is_plain_value(value)))?;
		let call = first_call(&body, local)?;
		let message = format!(
			"{written}(…) calls the local {written} = {}, which is no function: it hides the program's function {written}; rename the local",
			plain.serialize()
		);
		Some(Diagnostic::at(&call, message).into_error())
	})
}

fn assigned_values(body: &Node, local: &str) -> Vec<Node> {
	let mut values = vec![];
	body.visit(&mut |part| {
		if let Node::Key(target, Op::Assign, value) = part.drop_meta() {
			if target.symbol_name() == Some(local) {
				values.push(value.as_ref().clone());
			}
		}
	});
	values
}

fn is_plain_value(value: &Node) -> bool {
	matches!(
		value.drop_meta(),
		Node::True | Node::False | Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::List(_, Bracket::Square | Bracket::Curly, _)
	)
}

/// The first `local(…)` in the body, a call written with parentheses
fn first_call(body: &Node, local: &str) -> Option<Node> {
	let mut found = None;
	body.visit(&mut |part| {
		if found.is_none() {
			if let Node::List(items, Bracket::Round, _) = part.drop_meta() {
				if items.len() > 1 && items[0].symbol_name() == Some(local) {
					found = Some(part.clone());
				}
			}
		}
	});
	found
}
