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
/// The mark of the name taking the rest in `a, *rest = xs` (Python's starred target)
pub const STARRED: &str = "*";
/// The hidden names holding the value of a nested target: `(a, b), c = …`
const UNPACKED: &str = "unpacked";

/// `*rest` → `rest`, any other name as is
pub fn unstarred(name: &str) -> &str {
	name.strip_prefix(STARRED).unwrap_or(name)
}

pub fn lower(node: Node) -> Node {
	let lowered = match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
		Node::Key(left, Op::Assign, right) if bracketed_targets(&left).is_some() => {
			regroup_bracketed_destructuring(&left, &lower(*right)).expect("guarded")
		}
		Node::Key(left, op, right) => Node::Key(Box::new(lower(*left)), op, Box::new(lower(*right))),
		Node::List(items, bracket, separator) => {
			// `(a, b) = 1, 2` arrives as `((a, b) = 1), 2`: the first item stays an assignment for regroup_destructuring
			let takes_more_values = bracket == Bracket::None && separator == Separator::Colon;
			if takes_more_values {
				if let Err(error) = warn_tuple_comparison(&items) {
					return error;
				}
			}
			let lower_item = |(index, item): (usize, Node)| match item.drop_meta() {
				Node::Key(left, Op::Assign, right) if index == 0 && takes_more_values && bracketed_targets(left).is_some() => {
					Node::Key(left.clone(), Op::Assign, Box::new(lower(right.as_ref().clone())))
				}
				_ => lower(item),
			};
			let items: Vec<Node> = items.into_iter().enumerate().map(lower_item).collect();
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
		Node::Key(left, op, right) => Some(Node::Key(left.clone(), *op, Box::new(with_trailing_return(right, more)?))),
		Node::List(items, Bracket::None, Separator::Space) if items.len() == 2 && is_symbol(&items[0], RETURN) => {
			Some(Node::List(items.iter().chain(more).cloned().collect(), Bracket::None, Separator::Space))
		}
		_ => None,
	}
}

/// `x, (y = v), w` → `$destructure (x, y) v w`; `(x, y) = v, w` the same
fn regroup_destructuring(items: &[Node]) -> Option<Node> {
	let assignment = items.iter().position(|item| matches!(item.drop_meta(), Node::Key(_, Op::Assign, _)))?;
	let Node::Key(last_name, Op::Assign, first_value) = items[assignment].drop_meta() else { return None };
	let names: Vec<Node> = match (assignment, bracketed_targets(last_name)) {
		(0, Some(targets)) => targets.to_vec(),
		(0, None) => return None,
		_ => items[..assignment].iter().chain([last_name.as_ref()]).cloned().collect(),
	};
	if !names.iter().all(is_target) {
		return None;
	}
	let values: Vec<Node> = std::iter::once(first_value.as_ref()).chain(&items[assignment + 1..]).cloned().collect();
	let written = Node::List(items.to_vec(), Bracket::None, Separator::Colon).serialize();
	Some(destructure(names, values, written.trim(), ""))
}

/// `[a, b] = v` and `(a, b) = v`: the bracketed targets assigned one value
fn regroup_bracketed_destructuring(left: &Node, value: &Node) -> Option<Node> {
	let targets = bracketed_targets(left)?;
	Some(destructure(targets.to_vec(), vec![value.clone()], &format!("{} = {}", left.serialize(), value.serialize()), ""))
}

/// The targets of `(a, b)` / `[a, b]`: at least two names, starred names or nested targets, separated by commas
/// (`f(i) = …` defines f)
fn bracketed_targets(node: &Node) -> Option<&[Node]> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round | Bracket::Square, Separator::Colon) if items.len() >= 2 && items.iter().all(is_target) => Some(items),
		_ => None,
	}
}

fn is_target(node: &Node) -> bool {
	symbol_name(node).is_some() || bracketed_targets(node).is_some()
}

/// `$destructure (names) values…`; a nested target `(a, b)` takes a hidden name, unpacked by a following statement
fn destructure(names: Vec<Node>, values: Vec<Node>, written: &str, path: &str) -> Node {
	let starred = names.iter().filter(|name| symbol_name(name).is_some_and(|name| name.starts_with(STARRED))).count();
	if starred > 1 {
		return error("only one name may take the rest (`*rest`) in an unpacking");
	}
	// `a, *rest = 1, 2, 3` unpacks the list [1 2 3]; how many items the rest takes is known at run time
	let values = if starred == 1 && values.len() > 1 { vec![Node::List(values, Bracket::Square, Separator::Colon)] } else { values };
	if values.len() > 1 && values.len() != names.len() {
		return error(&format!("`{written}` gives {} values for {} names", values.len(), names.len()));
	}
	let mut nested = vec![];
	let names: Vec<Node> = names.into_iter().enumerate().map(|(index, name)| match bracketed_targets(&name) {
		Some(targets) => {
			let hidden = Node::Symbol(format!("{UNPACKED}{path}·{index}"));
			nested.push(destructure(targets.to_vec(), vec![hidden.clone()], written, &format!("{path}·{index}")));
			hidden
		}
		None => name,
	}).collect();
	let head = Node::Symbol(DESTRUCTURE.to_string());
	let names = Node::List(names, Bracket::Round, Separator::Colon);
	let statement = Node::List([head, names].into_iter().chain(values).collect(), Bracket::None, Separator::Space);
	if nested.is_empty() {
		return statement;
	}
	Node::List(std::iter::once(statement).chain(nested).collect(), Bracket::None, Separator::Semicolon)
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

/// `(a, b) == x, y` reads as `((a, b) == x), y`: the comma binds looser than `==` (user, P42 keeps that parse), so a tuple
/// compared with the first of several comma values gets a strong warning naming the parenthesized comparison
fn warn_tuple_comparison(items: &[Node]) -> Result<(), Node> {
	let Some(first) = items.first() else { return Ok(()) };
	let comparison = match first.drop_meta() {
		Node::Key(_, Op::Assign, value) => value.as_ref(),
		other => other,
	};
	let Node::Key(tuple, op @ (Op::Eq | Op::Ne), compared) = comparison.drop_meta() else { return Ok(()) };
	if !matches!(tuple.drop_meta(), Node::List(parts, Bracket::Round, Separator::Colon) if parts.len() > 1) {
		return Ok(());
	}
	let values: Vec<String> = std::iter::once(compared.as_ref()).chain(&items[1..]).map(Node::serialize).collect();
	let message = format!(
		"`{} {op} {}` compares the tuple with {} only, the comma binds looser than {op}: write `{} {op} ({})`",
		tuple.serialize(), values.join(", "), compared.serialize(), tuple.serialize(), values.join(", "));
	crate::diagnostic::report(&[crate::diagnostic::Diagnostic::at(first, message)])
}
