//! `dir(time)`, `dir text` (Python's dir, card dir-introspection): the names a standard module defines, a list of
//! texts in the order the module defines them. A program's own `dir`, or its own name of the module, wins

use crate::node::{Bracket, Node, Separator};

const DIR_WORD: &str = "dir";

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&[DIR_WORD]) {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	if defined.contains(DIR_WORD) {
		return node;
	}
	listed_names(node, &defined)
}

fn listed_names(node: Node, defined: &std::collections::HashSet<String>) -> Node {
	match node {
		Node::List(items, bracket @ (Bracket::Round | Bracket::None), separator) if items.len() == 2 && items[0].drop_meta().name() == DIR_WORD => {
			match module_names(&items[1], defined) {
				Some(names) => Node::List(names.iter().map(|name| Node::Text(name.clone())).collect(), Bracket::Square, Separator::None),
				None => Node::List(items.into_iter().map(|item| listed_names(item, defined)).collect(), bracket, separator),
			}
		}
		other => other.map_children(|child| listed_names(child, defined)),
	}
}

/// The names of the standard module `module` names, unless the program names something so itself
fn module_names(module: &Node, defined: &std::collections::HashSet<String>) -> Option<&'static [String]> {
	let Node::Symbol(name) = module.drop_meta() else { return None };
	if defined.contains(name) {
		return None;
	}
	crate::modules::std_module_definitions(name)
}
