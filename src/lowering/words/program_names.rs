//! What the program takes for itself, one answer for every pass (card one-dispatch, notes/dispatch.md "The rule",
//! levels 0 and 2): a word is the program's where one of its bindings is in scope. Everywhere: its functions, at the
//! numbers of arguments they take, and the variables it assigns at the top level. Inside a function: also its
//! parameters and the names its body assigns. A field of an object literal takes no bare word.

use super::nodes::{function_head, parameter_name};
use crate::node::Node;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Default)]
pub struct ProgramNames {
	/// Each function and the numbers of arguments its definitions take, fewest and most
	functions: HashMap<String, Vec<(usize, usize)>>,
	/// The variables bound where the names are asked: the top level's, a function's inside it
	bound: HashSet<String>,
}

impl ProgramNames {
	pub fn of(program: &Node) -> Self {
		let mut functions: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
		program.visit(&mut |part| {
			if let Some(head) = function_head(part) {
				functions.entry(head.name.to_string()).or_default().push(head.arity());
			}
		});
		let mut bound = HashSet::new();
		crate::type_tests::collect_outer_assigned_names(program, &mut bound);
		bound.extend(outer_loop_variables(program));
		bound.retain(|name| !functions.contains_key(name));
		ProgramNames { functions, bound }
	}

	/// Does the program take `word` here, as a function or a variable
	pub fn takes(&self, word: &str) -> bool {
		self.bound.contains(word) || self.functions.contains_key(word)
	}

	/// Does a variable or parameter here take `word`
	pub fn binds(&self, word: &str) -> bool {
		self.bound.contains(word)
	}

	/// Does the program take the call of `word` with that many arguments: a variable of that name, or a function
	/// taking them; `first(x) := …` leaves `first(xs, 3)` to the library
	pub fn takes_call(&self, word: &str, arguments: usize) -> bool {
		self.binds(word) || self.function_takes(word, arguments) == Some(true)
	}

	/// Does the program's function `word` take that many arguments; None when the program defines no function `word`
	pub fn function_takes(&self, word: &str, arguments: usize) -> Option<bool> {
		let arities = self.functions.get(word)?;
		Some(arities.iter().any(|(fewest, most)| (*fewest..=*most).contains(&arguments)))
	}

	/// The names in scope inside `node` when it defines a function: its parameters and the names its body assigns
	pub fn inside(&self, node: &Node) -> Option<ProgramNames> {
		let head = function_head(node)?;
		let mut inner = self.clone();
		inner.bound.extend(head.parameters().into_iter().filter_map(parameter_name));
		crate::library_words::collect_assigned_names(head.body, &mut inner.bound);
		inner.bound.extend(crate::for_loop::loop_variables(head.body));
		Some(inner)
	}
}

/// `rewrite` of a pass giving `word` its library or host meaning, applied where the program leaves the word free:
/// nowhere when the program takes it everywhere, and not inside a function binding it (`f(x) := { random = 4; … }`
/// keeps its own random, while the rest of the program has the host's)
pub fn rewritten_where_free(program: Node, word: &str, rewrite: &dyn Fn(Node) -> Node) -> Node {
	let names = ProgramNames::of(&program);
	where_free(program, &names, word, rewrite)
}

/// `rewrite` of every node bottom-up, given the names the program takes where the node stands
pub fn rewritten_by_scope(program: Node, rewrite: &dyn Fn(Node, &ProgramNames) -> Node) -> Node {
	let names = ProgramNames::of(&program);
	by_scope(program, &names, rewrite)
}

fn by_scope(node: Node, names: &ProgramNames, rewrite: &dyn Fn(Node, &ProgramNames) -> Node) -> Node {
	let inner = names.inside(&node);
	let names = inner.as_ref().unwrap_or(names);
	rewrite(node.map_children(|child| by_scope(child, names, rewrite)), names)
}

fn where_free(node: Node, names: &ProgramNames, word: &str, rewrite: &dyn Fn(Node) -> Node) -> Node {
	if names.takes(word) {
		return node;
	}
	if !binds_inside(&node, names, word) {
		return rewrite(node);
	}
	let inner = names.inside(&node);
	let names = inner.as_ref().unwrap_or(names);
	match inner.as_ref().map(|inner| inner.takes(word)) {
		Some(true) => node,
		_ => node.map_children(|child| where_free(child, names, word, rewrite)),
	}
}

/// Does a function inside `node` (itself too) bind `word`
fn binds_inside(node: &Node, names: &ProgramNames, word: &str) -> bool {
	let mut binds = false;
	node.visit(&mut |part| binds |= names.inside(part).is_some_and(|inner| inner.takes(word)));
	binds
}

/// `x = 1` of a parameter `x = 1`: a default value, which may be left out
/// The loop variables outside function bodies
fn outer_loop_variables(node: &Node) -> Vec<String> {
	if function_head(node).is_some() {
		return vec![];
	}
	let mut variables = match node.drop_meta() {
		Node::List(items, _, _) => super::nodes::for_in_variable(items).into_iter().collect(),
		_ => vec![],
	};
	variables.extend(node.drop_meta().parts().into_iter().flat_map(outer_loop_variables));
	variables
}
