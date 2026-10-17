//! `(p).a` is `p.a` (card dump-text): parentheses around a variable before a dot only group it, so the dot reads the
//! field or method as it does without them. Before, `(p).a` stayed the data `a:1.a` and `(t).dump` of a text failed
//! at run time as an item print could not join. `(function f).dump` keeps its group: the function itself, not a call.
//! The parser gives the call `f()` the same form as the group `(f)`, so only a name the program binds to a value (an
//! assignment, a loop variable) ungroups; any other name stays a call: `f().b`, `today().year`, `point().x`, `mk().a`
//! of `mk = () => …`, and the functions of an imported module

use crate::lowering::nodes::key;
use std::collections::HashSet;

use crate::node::{Bracket, Node};
use crate::operators::Op;

pub fn lower(node: Node) -> Node {
	let mut grouped = false;
	node.visit(&mut |part| {
		grouped |= matches!(part, Node::Key(receiver, Op::Dot, _) if grouped_name(receiver).is_some())
	});
	if !grouped {
		return node;
	}
	let variables = bound_variables(&node);
	ungrouped_receivers(node, &variables)
}

/// The names assigned a value that is no function, and the loop variables
fn bound_variables(program: &Node) -> HashSet<String> {
	let functions = crate::analyzer::function_definitions(program).user_functions;
	let mut variables: HashSet<String> = crate::lowering::for_loop::loop_variables(program).into_iter().collect();
	program.visit(&mut |part| {
		if let Node::Key(target, Op::Assign, value) = part {
			let is_function = crate::lowering::lambdas::arrow_lambda(value).is_some()
				|| crate::closures::referenced_function(value).is_some();
			if let Some(name) = target.symbol_name().filter(|_| !is_function) {
				variables.insert(name.to_string());
			}
		}
	});
	variables.retain(|name| !functions.contains_key(name));
	variables
}

fn ungrouped_receivers(node: Node, variables: &HashSet<String>) -> Node {
	match node {
		Node::Key(receiver, Op::Dot, member) => {
			let receiver = ungrouped_receivers(*receiver, variables);
			let receiver = match grouped_name(&receiver) {
				Some(name) if variables.contains(name.symbol_name().unwrap_or_default()) => name.clone(),
				_ => receiver,
			};
			key(receiver, Op::Dot, ungrouped_receivers(*member, variables))
		}
		other => other.map_children(|child| ungrouped_receivers(child, variables)),
	}
}

/// `(p)`: the one name in parentheses, no function reference
fn grouped_name(receiver: &Node) -> Option<&Node> {
	match receiver.drop_meta() {
		Node::List(items, Bracket::Round, _)
			if items.len() == 1
				&& items[0].symbol_name().is_some()
				&& crate::closures::referenced_function(&items[0]).is_none() =>
		{
			Some(&items[0])
		}
		_ => None,
	}
}
