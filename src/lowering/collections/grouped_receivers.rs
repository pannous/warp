//! `(p).a` is `p.a` (card dump-text): parentheses around a variable before a dot only group it, so the dot reads the
//! field or method as it does without them. Before, `(p).a` stayed the data `a:1.a` and `(t).dump` of a text failed
//! at run time as an item print could not join. `(function f).dump` keeps its group: the function itself, not a call.
//! The parser gives the call `f()` the same form as the group `(f)`, so a name that is a function stays a call:
//! `f().b`, `today().year`

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
	let functions: HashSet<String> = crate::analyzer::function_definitions(&node)
		.user_functions
		.into_keys()
		.collect();
	let is_function = |name: &str| {
		functions.contains(name)
			|| crate::library_words::is_library_word(name)
			|| crate::modules::std_module_defining(name).is_some()
	};
	ungrouped_receivers(node, &is_function)
}

fn ungrouped_receivers(node: Node, is_function: &dyn Fn(&str) -> bool) -> Node {
	match node {
		Node::Key(receiver, Op::Dot, member) => {
			let receiver = ungrouped_receivers(*receiver, is_function);
			let receiver = match grouped_name(&receiver) {
				Some(name) if !is_function(name.symbol_name().unwrap_or_default()) => name.clone(),
				_ => receiver,
			};
			Node::Key(
				Box::new(receiver),
				Op::Dot,
				Box::new(ungrouped_receivers(*member, is_function)),
			)
		}
		other => other.map_children(|child| ungrouped_receivers(child, is_function)),
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
