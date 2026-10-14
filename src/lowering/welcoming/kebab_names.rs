//! Kebab names: a data key read later in its block (`a-b:2 … a-b`), the difference of variables (`a-b`), and
//! `p.phone-number` of an object without that field

use crate::analyzer::{assigned_names, data_binding, function_definitions};
use crate::errors::diagnostic::Diagnostic;
use crate::lowering::nodes::children_rewritten;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

/// A symbol that is not a variable reads the value of the data key of that name given earlier in the same block:
/// `a-b:2 c-d:4 a-b` is 2 (the data is the scope)
pub fn resolve_data_scope(program: Node) -> Node {
	let mut variables = assigned_names(&program);
	variables.extend(function_definitions(&program).parameter_names());
	let resolved = resolve_blocks(program, &variables);
	subtract_kebab_variables(resolved, &variables)
}

/// field-or-minus (user 2026-10-12, replaced P168's subtraction): `p.phone-number` of an object without that field,
/// `number` a variable, is the error "p has no field phone-number" with the fix `p.phone - number`; an object whose
/// fields are unknown keeps the field read. Early, before member reads become indexes
pub fn lower_kebab_members(program: Node) -> Node {
	let objects = literal_object_fields(&program);
	if objects.is_empty() {
		return program;
	}
	let variables = assigned_names(&program);
	let mut missing = None;
	program.visit(&mut |node| {
		if let Node::Key(receiver, Op::Dot, member) = node {
			if missing.is_none() && is_missing_kebab_field(receiver, member, &variables, &objects) {
				missing = Some(node.clone());
			}
		}
	});
	match missing {
		Some(read) => missing_kebab_field(&read),
		None => program,
	}
}

/// "p has no field phone-number", offering the subtraction `p.phone - number`
fn missing_kebab_field(read: &Node) -> Node {
	let Node::Key(receiver, _, member) = read else {
		unreachable!("lower_kebab_members finds member reads");
	};
	let (object, name) = (receiver.drop_meta().name(), member.drop_meta().name());
	let subtraction = format!("{object}.{}", name.replace('-', " - "));
	Diagnostic::at(read, format!("{object} has no field {name}: write `{subtraction}` to subtract"))
		.offer("the subtraction", format!("{object}.{name}"), subtraction)
		.into_error()
}

/// The fields of the variables assigned object literals (`p = {phone: 7}`), over every such assignment
fn literal_object_fields(program: &Node) -> HashMap<String, HashSet<String>> {
	let mut objects: HashMap<String, HashSet<String>> = HashMap::new();
	program.visit(&mut |node| {
		let Node::Key(target, Op::Assign | Op::Define, value) = node else {
			return;
		};
		let (Node::Symbol(name), Node::List(entries, Bracket::Curly, _)) = (target.drop_meta(), value.drop_meta())
		else {
			return;
		};
		let keys: Option<Vec<String>> = entries
			.iter()
			.map(|entry| match entry.drop_meta() {
				Node::Key(key, Op::Colon, _) => Some(key.name()),
				_ => None,
			})
			.collect();
		if let Some(keys) = keys.filter(|keys| !keys.is_empty()) {
			objects.entry(name.clone()).or_default().extend(keys);
		}
	});
	objects
}

/// `p.a-b-c` where p's known fields lack `a-b-c` and b, c are variables: what reads like `p.a - b - c`
fn is_missing_kebab_field(
	receiver: &Node,
	member: &Node,
	variables: &HashSet<String>,
	objects: &HashMap<String, HashSet<String>>,
) -> bool {
	let (Node::Symbol(object), Node::Symbol(name)) = (receiver.drop_meta(), member.drop_meta()) else {
		return false;
	};
	let (Some(fields), Some((_, rest))) = (objects.get(object), name.split_once('-')) else {
		return false;
	};
	!fields.contains(name) && rest.split('-').all(|part| variables.contains(part))
}

/// A hyphenated name that is no data key and whose parts are all variables is their difference: `a=5; b=1; a-b` is 4.
/// The names of data keys and assignment targets stay as they are.
fn subtract_kebab_variables(node: Node, variables: &HashSet<String>) -> Node {
	match node {
		Node::Symbol(name) => {
			let parts: Vec<&str> = name.split('-').collect();
			if parts.len() > 1 && parts.iter().all(|part| variables.contains(*part)) {
				let mut terms = parts.into_iter().map(|part| Node::Symbol(part.to_string()));
				let first = terms.next().expect("split gives a part");
				terms.fold(first, |difference, term| Node::Key(Box::new(difference), Op::Sub, Box::new(term)))
			} else {
				Node::Symbol(name)
			}
		}
		Node::Key(left, op @ (Op::Colon | Op::Assign | Op::Define), right) => {
			Node::Key(left, op, Box::new(subtract_kebab_variables(*right, variables)))
		}
		other => children_rewritten(other, |child| subtract_kebab_variables(child, variables)),
	}
}

fn resolve_blocks(node: Node, variables: &HashSet<String>) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let mut keys: HashMap<String, Node> = HashMap::new();
			let mut resolved = Vec::with_capacity(items.len());
			let mut ends_in_lookup = false;
			for item in items {
				let item = resolve_blocks(item, variables);
				ends_in_lookup = matches!(item.drop_meta(), Node::Symbol(name) if keys.contains_key(name));
				let item = substitute_keys(item, &keys);
				if let Some((name, value)) = data_binding(&item) {
					if !variables.contains(name) {
						keys.insert(name.to_string(), value.clone());
					}
				}
				resolved.push(item);
			}
			match resolved.pop() {
				// only data collapses to the key it reads: `cat{…}; cat.kill!; cat` keeps running cat.kill!
				Some(value) if ends_in_lookup && resolved.iter().all(|item| data_binding(item).is_some()) => value,
				Some(last) => {
					resolved.push(last);
					Node::List(resolved, bracket, separator)
				}
				None => Node::List(resolved, bracket, separator),
			}
		}
		other => children_rewritten(other, |child| resolve_blocks(child, variables)),
	}
}

fn substitute_keys(node: Node, keys: &HashMap<String, Node>) -> Node {
	if keys.is_empty() {
		return node;
	}
	match node {
		Node::Symbol(name) => keys.get(&name).cloned().unwrap_or(Node::Symbol(name)),
		Node::Key(left, op @ (Op::Colon | Op::Assign | Op::Define), right) => {
			Node::Key(left, op, Box::new(substitute_keys(*right, keys)))
		}
		other => children_rewritten(other, |child| substitute_keys(child, keys)),
	}
}
