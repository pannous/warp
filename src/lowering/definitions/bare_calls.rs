//! A bare name of a function without parameters is its call (decision reflection, P82), also where its value is used:
//! `x = a` holds what a returns, `a + 1` adds to it (card assigning-bare). `function a` stays the function, a variable
//! or parameter of that name stays itself.

use super::nodes::{call, children_rewritten};
use crate::node::Node;
use crate::operators::Op;
use std::collections::HashSet;

pub fn lower(node: Node) -> Node {
	let parameterless = parameterless_functions(&node);
	if parameterless.is_empty() {
		return node;
	}
	rewrite(node, &parameterless)
}

/// The program's functions without parameters whose names no variable or parameter takes
fn parameterless_functions(node: &Node) -> HashSet<String> {
	let functions = crate::analyzer::function_definitions(node).user_functions;
	let assigned: HashSet<String> = node.assigned_names(|op| *op == Op::Assign);
	let parameters: HashSet<&str> =
		functions.values().flat_map(|function| function.params.iter().map(|param| param.name.as_str())).collect();
	functions
		.iter()
		.filter(|(name, function)| {
			function.params.is_empty() && !assigned.contains(*name) && !parameters.contains(name.as_str())
		})
		.map(|(name, _)| name.clone())
		.collect()
}

fn rewrite(node: Node, parameterless: &HashSet<String>) -> Node {
	let called = |side: Node| called_value(side, parameterless);
	match children_rewritten(node, |child| rewrite(child, parameterless)) {
		Node::Key(target, Op::Assign, value) => Node::Key(target, Op::Assign, Box::new(called(*value))),
		Node::Key(left, op, right) if op.is_operation() => {
			Node::Key(Box::new(called(*left)), op, Box::new(called(*right)))
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
