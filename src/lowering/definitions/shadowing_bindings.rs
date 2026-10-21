//! A local named like a function of the program is the local wherever the function binds it (cards local-named-like,
//! the family of binder-locals and program-var-leaks): `f() := { arity = [3]; arity in [[3]] }` tests the list,
//! `items[1..]` slices it, `g [1, 2]` and `g + 1` read it, whatever `arity(s)`, `items(p, closer)` or `g(x)` the
//! program defines. Each such local is renamed apart (`arity` → `arity·local`) before any pass reads a word as a
//! function's call; messages and wasm names show it as written (written_name). A call `g(3)` of a local holding a
//! plain value is the emitter's error for any variable applied to values (reject_variable_applied), naming the
//! function it hides.

use crate::lowering::nodes::{block, key, parameter_name};
use crate::modules::needed_definitions::renamed_words;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::program_names::{bound_inside, ProgramNames};
use crate::type_tests::function_definition;
use std::collections::HashMap;

const LOCAL_SUFFIX: &str = "·local";

pub fn lower(program: Node) -> Node {
	let names = ProgramNames::of(&program);
	apart(program, &names)
}

/// A name as the program wrote it: `arity·local` is `arity`
pub fn written_name(name: &str) -> &str {
	name.strip_suffix(LOCAL_SUFFIX).unwrap_or(name)
}

/// Is `name` a local renamed apart from the program's function of its written name
pub fn hides_function(name: &str) -> bool {
	name.ends_with(LOCAL_SUFFIX)
}

fn apart(node: Node, names: &ProgramNames) -> Node {
	let node = match local_renames(&node, names) {
		Some(renames) => renamed_apart(node, &renames),
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
		.filter(|local| *local != own_name && names.is_function(local))
		.map(|local| (local.clone(), format!("{local}{LOCAL_SUFFIX}")))
		.collect();
	(!renames.is_empty()).then_some(renames)
}

/// The definition with its body's locals renamed. A parameter keeps its name in the head, the function's interface
/// (`play(tone: 440Hz)` names it, as do `.params` and the messages): the body starts with its renamed copy,
/// `tone·local = tone`
fn renamed_apart(definition: Node, renames: &HashMap<String, String>) -> Node {
	let Some((head, body, rebuild)) = function_definition(&definition) else {
		return renamed_words(definition, renames);
	};
	let copies: Vec<Node> = crate::lowering::function_head(&definition)
		.map(|function| function.parameters().into_iter().filter_map(parameter_name).collect::<Vec<String>>())
		.unwrap_or_default()
		.into_iter()
		.filter_map(|parameter| {
			let local = renames.get(&parameter)?;
			Some(key(Node::Symbol(local.clone()), Op::Assign, Node::Symbol(parameter)))
		})
		.collect();
	rebuild(head, with_copies_first(renamed_words(body, renames), copies))
}

/// `{copies; body…}`: a block body takes them first, any other body becomes the block's last statement
fn with_copies_first(body: Node, copies: Vec<Node>) -> Node {
	if copies.is_empty() {
		return body;
	}
	match body {
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_copies_first(*node, copies)), data },
		Node::List(items, Bracket::Curly, separator @ (Separator::Semicolon | Separator::Newline)) => {
			Node::List([copies, items].concat(), Bracket::Curly, separator)
		}
		Node::List(items, Bracket::Curly, _) if items.len() == 1 => block([copies, items].concat()),
		other => block([copies, vec![other]].concat()),
	}
}
