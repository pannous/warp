//! A local named like a function of the program is the local wherever the function binds it (cards local-named-like,
//! the family of binder-locals and program-var-leaks): `f() := { arity = [3]; arity in [[3]] }` tests the list,
//! `items[1..]` slices it, `g [1, 2]` and `g + 1` read it, whatever `arity(s)`, `items(p, closer)` or `g(x)` the
//! program defines. Each such local is renamed apart (`arity` → `arity·local`) before any pass reads a word as a
//! function's call, parameters included; messages and wasm names show it as written (written_name), and callers name
//! a parameter as written (`pitch(tone: 4)` of `pitch(tone·local: int)`: named_arguments, preposition_calls). A call
//! `g(3)` of a local holding a plain value is the emitter's error for any variable applied to values
//! (reject_variable_applied), naming the function it hides.

use crate::modules::needed_definitions::renamed_words;
use crate::node::Node;
use crate::program_names::{bound_inside, ProgramNames};
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

/// Code as the program wrote it: `tone·local: Bus` is `tone: Bus`
pub fn written_text(code: &str) -> String {
	code.replace(LOCAL_SUFFIX, "")
}

/// Is `name` a local renamed apart from the program's function of its written name
pub fn hides_function(name: &str) -> bool {
	name.ends_with(LOCAL_SUFFIX)
}

fn apart(node: Node, names: &ProgramNames) -> Node {
	let node = match local_renames(&node, names) {
		Some(renames) => renamed_words(node, &renames),
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
