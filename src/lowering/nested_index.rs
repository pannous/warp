//! `m#2#1 = 9` (`m[1][0] = 9`) sets the item inside the inner list (card nested-index): an index assignment stores the
//! changed list back into its target, and `m#2` is no variable to store into. So the inner list is taken out, changed
//! and put back: `nested·0 = m#2; nested·0#1 = 9; m#2 = nested·0`, also for `+=` and deeper chains. An index that is a
//! computation is evaluated once.
//! A field of an element changes the same way (card list-field-receivers): `bags#1.n = 5`, `bags#1.items.add(v)` and
//! `bags#1.items#2 = v` take the instance out, change it and put it back. A store into a call's result
//! (`make().n = 5`) would change a copy nobody keeps, so it is an error.

use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const INNER: &str = "nested·";
const INDEX: &str = "nested·index·";

pub fn lower(node: Node) -> Node {
	let mut counter = 0;
	lowered(node, &mut counter)
}

fn lowered(node: Node, counter: &mut usize) -> Node {
	let Some(place) = changed_place(&node) else { return node.map_children(|child| lowered(child, counter)) };
	if let Some(call) = called_root(place) {
		let change = node.drop_meta().serialize();
		let message = format!("{call}() gives a copy: {change} changes nothing");
		return Diagnostic::at(&node, message).fix(format!("keep it first: x = {call}(); then change x")).into_error();
	}
	let Some(element) = element_below(place).cloned() else { return node.map_children(|child| lowered(child, counter)) };
	let number = *counter;
	*counter += 1;
	let Node::Key(base, Op::Hash, index) = element.drop_meta() else { unreachable!("element_below finds an index") };
	let (index_binding, index) = match index.drop_meta() {
		Node::Number(_) | Node::Symbol(_) => (None, index.as_ref().clone()),
		_ => {
			let name = Node::Symbol(format!("{INDEX}{number}"));
			(Some(assign(name.clone(), index.as_ref().clone())), name)
		}
	};
	let element = Node::Key(base.clone(), Op::Hash, Box::new(index));
	let inner = Node::Symbol(format!("{INNER}{number}"));
	let changed = with_element_replaced(node, &inner);
	let statements = index_binding.into_iter()
		.chain([assign(inner.clone(), element.clone()), lowered(changed, counter), lowered(assign(element, inner), counter)])
		.collect();
	Node::List(statements, Bracket::None, Separator::Semicolon)
}

/// The place a statement changes: the target of `t = v`, `t += v`, `t++`, or the list `l` of `l.add(v)`, `l.pop()`
fn changed_place(node: &Node) -> Option<&Node> {
	let Node::Key(target, op, value) = node.drop_meta() else { return None };
	if *op == Op::Assign || op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec) {
		return Some(target);
	}
	let Node::List(items, _, _) = value.drop_meta() else { return None };
	let changes_list = *op == Op::Dot && items.first().is_some_and(|method| crate::analyzer::is_list_mutating_method(&method.name()));
	changes_list.then_some(target)
}

/// The steps from a place down to what it changes inside: `x#i`, `x.field`
fn inside(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		Node::Key(left, Op::Hash, _) => Some(left),
		Node::Key(left, Op::Dot, field) if matches!(field.drop_meta(), Node::Symbol(_)) => Some(left),
		_ => None,
	}
}

/// The element `m#2` of `m#2#1`, `bags#1` of `bags#1.n`: the outermost index strictly inside the place
fn element_below(place: &Node) -> Option<&Node> {
	let mut node = inside(place)?;
	loop {
		if matches!(node.drop_meta(), Node::Key(_, Op::Hash, _)) {
			return Some(node);
		}
		node = inside(node)?;
	}
}

/// The function `make` of a place `make().n`, `make().items#1` inside a call's result
fn called_root(place: &Node) -> Option<String> {
	let mut node = inside(place)?;
	loop {
		if let Node::List(items, bracket, separator) = node.drop_meta() {
			return crate::analyzer::call_name(items, bracket, separator).map(str::to_string);
		}
		node = inside(node)?;
	}
}

/// The statement with the element inside its place (element_below) replaced by `inner`
fn with_element_replaced(node: Node, inner: &Node) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_element_replaced(*node, inner)), data },
		Node::Key(target, op, value) => Node::Key(Box::new(replaced_below(*target, inner)), op, value),
		other => other,
	}
}

fn replaced_below(place: Node, inner: &Node) -> Node {
	match place {
		Node::Meta { node, data } => Node::Meta { node: Box::new(replaced_below(*node, inner)), data },
		Node::Key(left, op, right) if inside(&Node::Key(left.clone(), op, right.clone())).is_some() => {
			let is_index = matches!(left.drop_meta(), Node::Key(_, Op::Hash, _));
			let left = if is_index { inner.clone() } else { replaced_below(*left, inner) };
			Node::Key(Box::new(left), op, right)
		}
		other => other,
	}
}

fn assign(target: Node, value: Node) -> Node {
	Node::Key(Box::new(target), Op::Assign, Box::new(value))
}
