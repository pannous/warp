//! Tuple returns (notes/open_decisions.md "Tuple returns", notes/multi_value.md): `return a, b` and `x, y = f()`.
//! The parser reads the comma loosest, so `f() := return 1, 2` arrives as the list `(f() := return 1), 2` and
//! `x, y = f()` as `x, (y = f())`; this pass regroups both:
//! - `return a, b` → `return a b` (one return with several values): a function returning that way returns them as wasm
//!   multi-value results, and a plain call `f()` gets them packed into the list `[a b]`
//! - `x, y = v` / `x, y = v, w` → `$destructure (x, y) v …` (a statement binding each name, values evaluated first)

use super::nodes::{is_word, key};
use crate::node::{error, symbol, Bracket, Node, Separator};
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
		Node::Key(left, op, right) => key(lower(*left), op, lower(*right)),
		Node::List(items, bracket, separator) => {
			// `(a, b) = 1, 2` arrives as `((a, b) = 1), 2`: the first item stays an assignment for regroup_destructuring
			let takes_more_values = bracket == Bracket::None && separator == Separator::Colon;
			if takes_more_values {
				if let Err(error) = refuse_comma_next_to_comparison(&items) {
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
				(Bracket::None, Separator::Colon) => regroup_return(&items).or_else(|| regroup_destructuring(&items)).or_else(|| regroup_declared_destructuring(&items)),
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
		Node::List(items, _, Separator::Space) if items.len() >= 3 && is_word(&items[0], RETURN) => Some(&items[1..]),
		_ => None,
	}
}

/// The names and values of `x, y = …` (lowered)
pub fn destructuring(node: &Node) -> Option<(Vec<String>, &[Node])> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let [head, names, values @ ..] = items.as_slice() else { return None };
	if !is_word(head, DESTRUCTURE) {
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
		Node::List(items, Bracket::None, Separator::Space) if items.len() == 2 && is_word(&items[0], RETURN) => {
			Some(Node::List(items.iter().chain(more).cloned().collect(), Bracket::None, Separator::Space))
		}
		_ => None,
	}
}

/// The variables a destructuring statement assigns: `x, y = v, w`, `(x, y) = v`, `a, *rest = …`
pub(crate) fn destructured_names(statement: &Node) -> Vec<String> {
	let targets: Vec<Node> = match statement.drop_meta() {
		Node::List(items, Bracket::None, Separator::Colon) => {
			let assignment = items.iter().position(|item| matches!(item.drop_meta(), Node::Key(_, Op::Assign, _)));
			match assignment.map(|at| (at, items[at].drop_meta())) {
				Some((at, Node::Key(last, _, _))) => items[..at].iter().chain([last.as_ref()]).cloned().collect(),
				_ => vec![],
			}
		}
		Node::Key(left, Op::Assign, _) => bracketed_targets(left).map(<[Node]>::to_vec).unwrap_or_default(),
		_ => vec![],
	};
	let mut names = vec![];
	if targets.iter().all(is_target) {
		targets.iter().for_each(|target| target.visit(&mut |part| names.extend(symbol_name(part).map(|name| name.trim_start_matches(STARRED).to_string()))));
	}
	names
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

/// `let a, b = 3, 4` → `let ($destructure (a, b) 3 4)`, like `let (a, b) = 3, 4`: Python's unpacking, assumed (card
/// let-comma; JS would declare a without a value and b = 3: notes/open_decisions.md)
fn regroup_declared_destructuring(items: &[Node]) -> Option<Node> {
	let (first, rest) = items.split_first()?;
	let Node::List(declared, Bracket::None, Separator::Space) = first.drop_meta() else { return None };
	let [keyword, target] = declared.as_slice() else { return None };
	if !crate::analyzer::is_declaration_keyword(keyword) {
		return None;
	}
	let targets: Vec<Node> = std::iter::once(target.clone()).chain(rest.iter().cloned()).collect();
	let destructuring = regroup_destructuring(&targets)?;
	Some(Node::List(vec![keyword.clone(), destructuring], Bracket::None, Separator::Space))
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
	let head = symbol(DESTRUCTURE);
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
			if items.len() == 2 && is_word(&items[0], RETURN) {
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

/// `a == b` among comma values (the first may assign it, `x = a == b, c`): its position, item, sides and operator
fn comparison_in(index: usize, item: &Node) -> Option<(usize, &Node, &Node, Op, &Node)> {
	let comparison = match item.drop_meta() {
		Node::Key(_, Op::Assign, value) if index == 0 => value.as_ref(),
		other => other,
	};
	match comparison.drop_meta() {
		Node::Key(left, op @ (Op::Eq | Op::Ne), right) => Some((index, item, left.as_ref(), *op, right.as_ref())),
		_ => None,
	}
}

/// `(a, b) == x, y` reads as `((a, b) == x), y`, `a, b == c` as `a, (b == c)`: the comma binds looser than `==` (P42),
/// which misleads, so a comma next to a comparison without parentheses is an error asking for them (user, P227: "insist
/// on braces to avoid errors"), with both parenthesized readings as fixes
fn refuse_comma_next_to_comparison(items: &[Node]) -> Result<(), Node> {
	let Some((index, item, left, op, right)) = items.iter().enumerate().find_map(|(index, item)| comparison_in(index, item)) else { return Ok(()) };
	let before: Vec<&Node> = items[..index].iter().chain(std::iter::once(left)).collect();
	let after: Vec<&Node> = std::iter::once(right).chain(&items[index + 1..]).collect();
	let text = |node: &Node| node.serialize().trim().to_string();
	let joined = |nodes: Vec<&Node>| nodes.into_iter().map(text).collect::<Vec<_>>().join(", ");
	let (single_before, single_after) = (before.len() == 1, after.len() == 1);
	let (before, after) = (joined(before), joined(after));
	let group = |text: &str, single: bool| if single { text.to_string() } else { format!("({text})") };
	let written = format!("{before} {op} {after}");
	let all_values = format!("{} {op} {}", group(&before, single_before), group(&after, single_after));
	let comparison = format!("({} {op} {})", text(left), text(right));
	let apart = items[..index].iter().map(text).chain(std::iter::once(comparison)).chain(items[index + 1..].iter().map(text)).collect::<Vec<_>>().join(", ");
	let message = format!("`{written}`: a comma next to {op} needs parentheses, the comma binds looser than {op}: write `{all_values}` or `{apart}`");
	Err(crate::diagnostic::Diagnostic::at(item, message)
		.offer("compare all the values", written.clone(), all_values)
		.offer("keep the comparison apart", written, apart)
		.into_error())
}
