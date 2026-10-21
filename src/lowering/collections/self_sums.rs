//! `out = out + x` of a list local nothing else holds grows it in place, as `out += x` does (card out-list): copying the
//! whole list at each step was quadratic. `+=` changes the list for every holder (P200b), so only a local of a function
//! that cannot have another holder is rewritten: given only list literals and its own sums, read only where no
//! reference to it is kept (an index, a method call, an operand, a counting word), and the body's value only at its end.

use crate::lowering::memoization::definition_parts;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;

/// The words reading a list without keeping it
const READING_WORDS: [&str; 7] = ["count", "size", "sum", "min", "max", "join", "print"];

pub fn lower(node: Node) -> Node {
	let mut outer = HashSet::new();
	crate::lowering::type_tests::collect_outer_assigned_names(&node, &mut outer);
	lowered(node, &outer)
}

fn lowered(node: Node, outer: &HashSet<String>) -> Node {
	if let Some((head, body, rebuild)) = definition_parts(&node) {
		let names: Vec<String> = self_summed_names(&body)
			.into_iter()
			.filter(|name| !outer.contains(name) && !mentions(&head, name))
			.collect();
		let body =
			names.iter().filter(|name| stays_unshared(&body, name, true)).fold(body, |body, name| in_place(body, name));
		return rebuild(head, lowered(body, outer));
	}
	crate::node::children_rewritten(node, |child| lowered(child, outer))
}

/// The names of `x = x + e` in the body
fn self_summed_names(body: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	body.visit(&mut |part| {
		if let Some(name) = self_sum(part).map(|(name, _)| name) {
			names.insert(name.to_string());
		}
	});
	names
}

/// `x = x + e`: x and e
fn self_sum(node: &Node) -> Option<(&str, &Node)> {
	let Node::Key(target, Op::Assign, value) = node.drop_meta() else { return None };
	let name = target.symbol_name()?;
	match value.ungrouped().drop_meta() {
		Node::Key(left, Op::Add, added) if left.symbol_name() == Some(name) => Some((name, added)),
		_ => None,
	}
}

fn is(node: &Node, name: &str) -> bool {
	node.symbol_name() == Some(name)
}

fn mentions(node: &Node, name: &str) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= is(part, name));
	found
}

/// Whether no reference to the list `name` outlives its use in `node`; `last` when node gives the body's value
fn stays_unshared(node: &Node, name: &str, last: bool) -> bool {
	let unshared = |part: &Node| stays_unshared(part, name, false);
	match node.drop_meta() {
		symbol @ Node::Symbol(_) => !is(symbol, name) || last,
		// a lambda may keep the list it mentions
		Node::Key(_, Op::Arrow, _) => !mentions(node, name),
		Node::Key(target, Op::Assign | Op::Define, value) if is(target, name) => {
			let fresh = value.ungrouped();
			match fresh.drop_meta() {
				Node::Key(left, Op::Add, added) if is(left, name) => unshared(added),
				Node::List(items, Bracket::Square, _) => items.iter().all(unshared),
				_ => value.is_empty_list_literal() || fresh.is_empty_list_literal(),
			}
		}
		Node::Key(target, Op::AddAssign, added) if is(target, name) => unshared(added),
		Node::Key(target, op, _) if is(target, name) && op.is_compound_assign() => false,
		Node::Key(empty, Op::Hash, counted) if empty.is_empty_node() && is(counted, name) => true, // `#out`
		// `out#i`, `out.add(v)`, `out + [v]`, `out == []`: read where it stands
		Node::Key(left, op, right)
			if is(left, name) && (matches!(op, Op::Hash | Op::Dot) || op.is_arithmetic() || op.is_comparison()) =>
		{
			unshared(right)
		}
		Node::Key(left, op, right) if is(right, name) && (op.is_arithmetic() || op.is_comparison()) => unshared(left),
		Node::Key(left, _, right) => unshared(left) && unshared(right),
		Node::List(items, _, _) if is_reading_call(items, name) => true,
		Node::List(items, bracket, Separator::Semicolon | Separator::Newline) if *bracket != Bracket::Square => {
			let tail = items.len().saturating_sub(1);
			items.iter().enumerate().all(|(index, item)| stays_unshared(item, name, last && index == tail))
		}
		Node::List(items, _, _) => items.iter().all(unshared),
		_ => true,
	}
}

/// `count(out)`, `join(out, ",")`: a word reading the list
fn is_reading_call(items: &[Node], name: &str) -> bool {
	match items {
		[word, argument, rest @ ..] => {
			READING_WORDS.contains(&word.drop_meta().name().as_str())
				&& is(argument, name)
				&& rest.iter().all(|part| stays_unshared(part, name, false))
		}
		_ => false,
	}
}

/// Every `name = name + e` of the body as `name += e`
fn in_place(node: Node, name: &str) -> Node {
	let summed = !matches!(node, Node::Meta { .. }) && self_sum(&node).is_some_and(|(summed, _)| summed == name);
	if let Some((_, added)) = self_sum(&node).filter(|_| summed) {
		let added = added.clone();
		return Node::Key(Box::new(Node::Symbol(name.to_string())), Op::AddAssign, Box::new(in_place(added, name)));
	}
	crate::node::children_rewritten(node, |child| in_place(child, name))
}
