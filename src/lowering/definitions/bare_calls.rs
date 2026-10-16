//! A bare name of a function without parameters is its call (decision reflection, P82), also where its value is used:
//! `x = a` holds what a returns, `a + 1` adds to it (card assigning-bare). `function a` stays the function, a variable
//! or parameter of that name stays itself. A word of the host taking nothing, `random`, is called the same way, and also
//! where the emitter calls a program's function by itself: as a statement, after `print` and `type` (card random-should).

use super::nodes::{call, children_rewritten, key};
use crate::lowering::reflection::CALLING_WORDS;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::HashSet;

/// The names a bare word calls: every function without parameters, and of them the host's
struct Parameterless {
	all: HashSet<String>,
	host: HashSet<String>,
}

pub fn lower(node: Node) -> Node {
	let parameterless = parameterless_functions(&node);
	if parameterless.all.is_empty() {
		return node;
	}
	let node = rewrite(node, &parameterless);
	called_value(node, &parameterless.host)
}

/// The functions without parameters, the program's and the host's, whose names no variable or parameter takes
fn parameterless_functions(node: &Node) -> Parameterless {
	let functions = crate::analyzer::function_definitions(node).user_functions;
	let assigned: HashSet<String> = node.assigned_names(|op| *op == Op::Assign);
	let parameters: HashSet<&str> =
		functions.values().flat_map(|function| function.params.iter().map(|param| param.name.as_str())).collect();
	let free = |name: &str| !assigned.contains(name) && !parameters.contains(name);
	let host: HashSet<String> = crate::host::parameterless_host_words()
		.filter(|name| free(name) && !functions.contains_key(*name))
		.map(String::from)
		.collect();
	let program = functions.iter().filter(|(name, function)| function.params.is_empty() && free(name));
	let all = program.map(|(name, _)| name.clone()).chain(host.iter().cloned()).collect();
	Parameterless { all, host }
}

fn rewrite(node: Node, parameterless: &Parameterless) -> Node {
	let called = |side: Node| called_value(side, &parameterless.all);
	let host_called = |side: Node| called_value(side, &parameterless.host);
	match children_rewritten(node, |child| rewrite(child, parameterless)) {
		Node::Key(target, Op::Assign, value) => Node::Key(target, Op::Assign, Box::new(called(*value))),
		Node::Key(left, op, right) if op.is_operation() => key(called(*left), op, called(*right)),
		// `random as int`: the value cast, never the type
		Node::Key(value, Op::As, type_name) => Node::Key(Box::new(called(*value)), Op::As, type_name),
		Node::List(items, bracket, separator) if separator.separates_statements() => {
			Node::List(items.into_iter().map(host_called).collect(), bracket, separator)
		}
		Node::List(items, bracket @ (Bracket::Round | Bracket::None), separator) if matches!(items.as_slice(), [word, _] if CALLING_WORDS.iter().any(|calling| word.is_symbol(calling))) =>
		{
			let mut items = items;
			let argument = host_called(items.pop().expect("the argument"));
			items.push(argument);
			Node::List(items, bracket, separator)
		}
		other => other,
	}
}

/// `a()` of the bare name a, written without `function`
fn called_value(value: Node, parameterless: &HashSet<String>) -> Node {
	match value.drop_meta().symbol_name().map(String::from) {
		Some(name) if parameterless.contains(&name) && crate::closures::referenced_function(&value).is_none() => {
			call(&name, vec![])
		}
		_ => value,
	}
}
