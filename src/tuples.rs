//! Tuple returns (notes/open_decisions.md "Tuple returns", notes/multi_value.md): `return a, b` and `x, y = f()`.
//! The parser reads the comma loosest, so `f() := return 1, 2` arrives as the list `(f() := return 1), 2` and
//! `x, y = f()` as `x, (y = f())`; this pass regroups both:
//! - `return a, b` → `return a b` (one return with several values): a function returning that way returns them as wasm
//!   multi-value results, and a plain call `f()` gets them packed into the list `[a b]`
//! - `x, y = v` / `x, y = v, w` → `$destructure (x, y) v …` (a statement binding each name, values evaluated first)

use crate::node::{error, Bracket, Node, Separator};
use crate::operators::Op;

pub const RETURN: &str = "return";
pub const DESTRUCTURE: &str = "$destructure";

pub fn lower(node: Node) -> Node {
	let lowered = match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
		Node::Key(left, op, right) => Node::Key(Box::new(lower(*left)), op, Box::new(lower(*right))),
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(lower).collect();
			let regrouped = match (&bracket, &separator) {
				(Bracket::None, Separator::Colon) => regroup_return(&items).or_else(|| regroup_destructuring(&items)),
				// `{ return a, b }` keeps its one statement; `{a, b=2}` stays data
				(Bracket::Curly, Separator::Colon) => regroup_return(&items).map(|statement| Node::List(vec![statement], Bracket::Curly, Separator::Semicolon)),
				_ => None,
			};
			regrouped.unwrap_or(Node::List(items, bracket, separator))
		}
		other => other,
	};
	check_definition(&lowered).unwrap_or(lowered)
}

/// The values of `return a, b, …` (lowered), or None for any other node
pub fn returned_values(node: &Node) -> Option<&[Node]> {
	match node.drop_meta() {
		Node::List(items, _, Separator::Space) if items.len() >= 3 && is_symbol(&items[0], RETURN) => Some(&items[1..]),
		_ => None,
	}
}

/// The names and values of `x, y = …` (lowered)
pub fn destructuring(node: &Node) -> Option<(Vec<String>, &[Node])> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let [head, names, values @ ..] = items.as_slice() else { return None };
	if !is_symbol(head, DESTRUCTURE) {
		return None;
	}
	let Node::List(names, _, _) = names.drop_meta() else { return None };
	Some((names.iter().filter_map(symbol_name).collect(), values))
}

/// How many values a function body returns with `return a, b`; None when every return gives one value
pub fn tuple_arity(body: &Node) -> Option<usize> {
	let mut arity = None;
	body.visit(&mut |node| {
		if let Some(values) = returned_values(node) {
			arity = arity.max(Some(values.len()));
		}
	});
	arity
}

/// `f(args)` / `f args` / `f()`: the callee and the arguments; whether `f` returns several values is the caller's question
pub fn call_parts(node: &Node) -> Option<(&str, &[Node])> {
	match node.drop_meta() {
		Node::List(items, _, _) => match items.first()?.drop_meta() {
			Node::Symbol(name) => Some((name.as_str(), &items[1..])),
			_ => None,
		},
		_ => None,
	}
}

/// The wasm name of the function packing f's values into a list
pub fn packer_name(function: &str) -> String {
	format!("{function}$list")
}

/// Kind key of a tuple function's i-th value among the function kinds: `divmod#1`
pub fn element_key(function: &str, index: usize) -> String {
	format!("{function}#{index}")
}

fn symbol_name(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		_ => None,
	}
}

fn is_symbol(node: &Node, word: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == word)
}

/// `(… return a), b, c` → `… return a b c`: the return at the end of the first item takes the other items
fn regroup_return(items: &[Node]) -> Option<Node> {
	let (first, rest) = items.split_first()?;
	if rest.is_empty() {
		return None;
	}
	with_trailing_return(first, rest)
}

fn with_trailing_return(node: &Node, more: &[Node]) -> Option<Node> {
	match node {
		Node::Meta { node, data } => Some(Node::Meta { node: Box::new(with_trailing_return(node, more)?), data: data.clone() }),
		Node::Key(left, op, right) => Some(Node::Key(left.clone(), op.clone(), Box::new(with_trailing_return(right, more)?))),
		Node::List(items, Bracket::None, Separator::Space) if items.len() == 2 && is_symbol(&items[0], RETURN) => {
			Some(Node::List(items.iter().chain(more).cloned().collect(), Bracket::None, Separator::Space))
		}
		_ => None,
	}
}

/// `x, (y = v), w` → `$destructure (x, y) v w`
fn regroup_destructuring(items: &[Node]) -> Option<Node> {
	let assignment = items.iter().position(|item| matches!(item.drop_meta(), Node::Key(_, Op::Assign, _)))?;
	let Node::Key(last_name, Op::Assign, first_value) = items[assignment].drop_meta() else { return None };
	let names: Vec<Node> = items[..assignment].iter().chain([last_name.as_ref()]).cloned().collect();
	if assignment == 0 || !names.iter().all(|name| symbol_name(name).is_some()) {
		return None;
	}
	let values: Vec<Node> = std::iter::once(first_value.as_ref()).chain(&items[assignment + 1..]).cloned().collect();
	if values.len() > 1 && values.len() != names.len() {
		let written = Node::List(items.to_vec(), Bracket::None, Separator::Colon).serialize();
		return Some(error(&format!("`{}` gives {} values for {} names", written.trim(), values.len(), names.len())));
	}
	let head = Node::Symbol(DESTRUCTURE.to_string());
	let names = Node::List(names, Bracket::Round, Separator::Colon);
	Some(Node::List([head, names].into_iter().chain(values).collect(), Bracket::None, Separator::Space))
}

/// A function that returns several values does so on every return, and ends with such a return
fn check_definition(node: &Node) -> Option<Node> {
	let Node::Key(left, Op::Define | Op::Assign, body) = node.drop_meta() else { return None };
	let name = match left.drop_meta() {
		Node::List(items, _, _) => symbol_name(items.first()?)?,
		_ => symbol_name(left)?,
	};
	let arity = tuple_arity(body)?;
	let mut single = None;
	body.visit(&mut |node| {
		if let Node::List(items, _, _) = node.drop_meta() {
			if items.len() == 2 && is_symbol(&items[0], RETURN) {
				single.get_or_insert_with(|| node.serialize());
			}
		}
	});
	if let Some(written) = single {
		return Some(error(&format!("{name} returns {arity} values with `return a, b`, but `{}` returns one", written.trim())));
	}
	let ends_with_return = match body.drop_meta() {
		Node::List(statements, Bracket::Curly, _) => statements.last().is_some_and(|last| returned_values(last).is_some()),
		other => returned_values(other).is_some(),
	};
	(!ends_with_return).then(|| error(&format!("{name} returns {arity} values, so it must end with `return` and {arity} values")))
}
