//! Builtins `min(a, b, …)` and `max(a, b, …)`: lowered to comparisons, so exact and float operands
//! share the arithmetic emitters. Arguments are evaluated twice, so only side-effect free ones are accepted.
//! One list argument is the list of candidates: `max([1 5 2])`; a list variable `max(xs)` is folded at runtime.

use crate::analyzer::{call_name, extract_user_functions};
use crate::context::Context;
use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::wasp_parser::parse;
use crate::operators::Op;

const EXTREMA: [(&str, Op); 2] = [("min", Op::Lt), ("max", Op::Gt)];
const MIN_ARGUMENTS: usize = 2;

/// Pseudo-call the emitter turns into the runtime error `<extremum> of an empty list`
pub const EMPTY_EXTREMUM_CALL: &str = "empty_extremum";
pub const EMPTY_LIST_ERRORS: [(&str, &str); 2] = [("min", "min_of_an_empty_list"), ("max", "max_of_an_empty_list")];

pub fn lower(node: Node) -> Node {
	let mut context = Context::new();
	extract_user_functions(&mut context, &node);
	let builtins: Vec<(&str, Op)> = EXTREMA.into_iter().filter(|(name, _)| !context.user_functions.contains_key(*name)).collect();
	if builtins.is_empty() {
		return node;
	}
	expand(node, &builtins)
}

fn expand(node: Node, builtins: &[(&str, Op)]) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| expand(item, builtins)).collect();
			extremum(&items, &bracket, &separator, builtins).unwrap_or(Node::List(items, bracket, separator))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(expand(*left, builtins)), op, Box::new(expand(*right, builtins))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(expand(*node, builtins)), data },
		other => other,
	}
}

fn extremum(items: &[Node], bracket: &Bracket, separator: &Separator, builtins: &[(&str, Op)]) -> Option<Node> {
	let name = call_name(items, bracket, separator)?;
	let (_, better) = builtins.iter().find(|(builtin, _)| *builtin == name)?;
	if let [Node::Symbol(list)] = items.get(1..).unwrap_or_default().iter().map(Node::drop_meta).collect::<Vec<_>>().as_slice() {
		return Some(fold_list_at_runtime(name, list, better));
	}
	let listed = list_literal_items(&items[1..]);
	let arguments = listed.unwrap_or(&items[1..]);
	if arguments.is_empty() {
		return Some(Diagnostic::at(&items[0], format!("{name} of an empty list")).into_error());
	}
	if arguments.len() < MIN_ARGUMENTS && listed.is_none() {
		return Some(Diagnostic::at(&items[0], format!("{name} takes at least {MIN_ARGUMENTS} arguments, got {}", arguments.len())).into_error());
	}
	if !arguments.iter().all(is_side_effect_free) {
		return Some(Diagnostic::at(&items[0], format!("{name} arguments must be plain values or arithmetic: they are evaluated twice")).into_error());
	}
	let chosen = arguments[1..].iter().fold(arguments[0].clone(), |best, next| {
		let comparison = Node::Key(Box::new(best.clone()), *better, Box::new(next.clone()));
		let branches = Node::Key(Box::new(best), Op::Colon, Box::new(next.clone()));
		Node::Key(Box::new(comparison), Op::Question, Box::new(branches))
	});
	Some(chosen)
}

/// The items of the one list literal given instead of several arguments; ø is the empty list
fn list_literal_items(arguments: &[Node]) -> Option<&[Node]> {
	match arguments {
		[single] => match single.drop_meta() {
			Node::List(items, Bracket::Square, _) => Some(items),
			Node::Empty => Some(&[]),
			_ => None,
		},
		_ => None,
	}
}

/// `max(xs)` for a list variable: the first item, improved by every later one; an empty list is an error
fn fold_list_at_runtime(name: &str, list: &str, better: &Op) -> Node {
	parse(&format!(
		"(if count({list}) == 0 then {EMPTY_EXTREMUM_CALL}({name}) else (extremum_best={list}#1; for extremum_item in {list} {{ extremum_best = if extremum_item {better} extremum_best then extremum_item else extremum_best }}; extremum_best))"
	))
}

fn is_side_effect_free(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(_) | Node::Symbol(_) | Node::True | Node::False | Node::Empty => true,
		Node::Key(left, op, right) => {
			(op.is_arithmetic() || op.is_comparison() || matches!(op, Op::Question | Op::Colon)) && is_side_effect_free(left) && is_side_effect_free(right)
		}
		_ => false,
	}
}
