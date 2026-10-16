//! `{a:1}` stays its pair at run time (user, card single-entry, notes/decisions.md) and becomes the map of that one entry
//! where a map is needed: an untyped or map-typed argument of the program's own function (`kind_of({a:1})` tests `x is map`), a variable a
//! map method changes (`m = {a:1}; m.pop("a")`, `m.add(…)`, `m += …`, or one passed to such a function) and the entry
//! appended into a map (`m += {b:2}`). Reading it as a map works on the pair as it is: count, keys, values, get, has,
//! `for k in m`, `m["b"] = 2`.

use super::memoization::definition_parts;
use super::nodes::{call, is_colon_pair, key};
use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::words::MAP_WORD;
use std::collections::{HashMap, HashSet};

/// `map_of_entry(x)`: the lone entry x as the map of it, any other value itself (wasm_emitter list_ops.rs)
pub const MAP_OF_ENTRY: &str = "map_of_entry";
/// The methods changing a map in place, which a lone pair can't take
const CHANGING_METHODS: [&str; 9] = ["pop", "remove", "delete", "add", "append", "push", "insert", "put", "set"];

pub fn lower(node: Node) -> Node {
	let functions = defined_functions(&node);
	let changed = changed_maps(&node, &functions);
	if functions.is_empty() && changed.is_empty() {
		return node;
	}
	upgraded(node, &functions, &changed)
}

/// `{a:1}`: a braced single pair
fn is_one_entry(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, Bracket::Curly, _) if matches!(items.as_slice(), [entry] if is_colon_pair(entry)))
}

fn as_map(node: Node) -> Node {
	if is_one_entry(&node) {
		call(MAP_OF_ENTRY, vec![node])
	} else {
		node
	}
}

/// The program's functions, each with whether its parameters take a map: an untyped `x` or `m:map` does, `p:photo`
/// does not (a typed parameter checks the fields of the pair it is given, tests/operators/test_like.rs)
type Functions = HashMap<String, Vec<bool>>;

fn takes_map(parameter: &Node) -> bool {
	match parameter.drop_meta() {
		Node::Key(_, Op::Colon, type_name) => type_name.drop_meta().name().starts_with(MAP_WORD),
		_ => true,
	}
}

fn defined_functions(program: &Node) -> Functions {
	let mut functions = Functions::new();
	program.visit(&mut |node| {
		if let Some((head, _, _)) = definition_parts(node) {
			if let Node::List(items, _, _) = head.drop_meta() {
				if let Some(name) = items.first().and_then(Node::symbol_name) {
					functions.insert(name.to_string(), items[1..].iter().map(takes_map).collect());
				}
			}
		}
	});
	functions
}

/// The arguments of a call of one of the program's functions that take a map: `f(a)` and `f(a, b)`
fn map_arguments<'a>(node: &'a Node, functions: &Functions) -> Option<Vec<(&'a Node, bool)>> {
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() > 1 => {
			let takes = functions.get(items[0].symbol_name()?)?;
			Some(
				items[1..]
					.iter()
					.enumerate()
					.map(|(index, argument)| (argument, takes.get(index).copied().unwrap_or(false)))
					.collect(),
			)
		}
		_ => None,
	}
}

/// `m.pop(k)`, `m.add(x)`: the receiver of a changing map method
fn changing_receiver(node: &Node) -> Option<&str> {
	match node.drop_meta() {
		Node::Key(receiver, Op::Dot, member) => {
			let method = match member.drop_meta() {
				Node::List(items, _, _) => items.first().and_then(Node::symbol_name),
				other => other.symbol_name(),
			};
			method.filter(|method| CHANGING_METHODS.contains(method)).and(receiver.symbol_name())
		}
		Node::Key(receiver, Op::AddAssign, _) => receiver.symbol_name(),
		_ => None,
	}
}

/// The variables a map method changes or the program's functions take
fn changed_maps(program: &Node, functions: &Functions) -> HashSet<String> {
	let mut changed = HashSet::new();
	program.visit(&mut |node| {
		changed.extend(changing_receiver(node).map(str::to_string));
		let arguments = map_arguments(node, functions).unwrap_or_default();
		changed.extend(
			arguments
				.into_iter()
				.filter(|(_, takes)| *takes)
				.filter_map(|(argument, _)| argument.symbol_name())
				.map(str::to_string),
		);
	});
	changed
}

fn upgraded(node: Node, functions: &Functions, changed: &HashSet<String>) -> Node {
	let takes: Option<Vec<bool>> =
		map_arguments(&node, functions).map(|arguments| arguments.into_iter().map(|(_, takes)| takes).collect());
	match node {
		Node::Key(target, op @ (Op::Assign | Op::AddAssign), value)
			if target.symbol_name().is_some_and(|name| changed.contains(name)) =>
		{
			key(*target, op, as_map(upgraded(*value, functions, changed)))
		}
		Node::List(items, bracket, separator) if takes.is_some() => {
			let takes = takes.unwrap_or_default();
			let mut items = items.into_iter().map(|item| upgraded(item, functions, changed));
			let head = items.next().into_iter();
			let arguments =
				items.zip(takes).map(|(argument, takes_map)| if takes_map { as_map(argument) } else { argument });
			Node::List(head.chain(arguments).collect(), bracket, separator)
		}
		other => other.map_children(|child| upgraded(child, functions, changed)),
	}
}
