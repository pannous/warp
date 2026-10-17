//! Templates (card data-splice, notes/quasi_quote.md): quoted code whose holes splice values, the other half of code
//! patterns. `$a` in `data($a + 1)` is the value of a, `$(expr)` the value of expr and `$xs...` the items of the list xs;
//! the template is built where it is written with `key`, `bracketed` and `separated` (reflection/node_api.rs), so every
//! later pass sees the values spliced as the code they are. Where a pattern stands, `data(…)` is a pattern whose holes
//! bind (switch.rs): an arm's key with its guard, the right side of `is` and `==`; tree walks are match arms by now.

use super::blocks::{quote, quoted_data};
use super::nodes::children_rewritten;
use super::switch::{hole_call, hole_name, rest_hole};
use crate::node::{call, text, Bracket, Node, Separator};
use crate::operators::Op;

const KEY_BUILDER: &str = "key";
const BRACKETED_BUILDER: &str = "bracketed";
const SEPARATED_BUILDER: &str = "separated";
/// `$(expr)`: the parser's call of `$`
const SPLICE_CALL: &str = "$";

pub fn lower(node: Node) -> Node {
	if let Some(code) = quoted_data(&node) {
		return if has_splice(code) { built(code) } else { node };
	}
	match node {
		Node::Key(pattern, op @ (Op::Colon | Op::Arrow), body) => Node::Key(pattern, op, Box::new(lower(*body))),
		Node::Key(subject, Op::Eq, pattern) => Node::Key(Box::new(lower(*subject)), Op::Eq, pattern),
		meta @ Node::Meta { .. } => meta.under_meta(lower),
		other => children_rewritten(other, lower),
	}
}

fn has_splice(code: &Node) -> bool {
	spliced(code).is_some()
		|| match code.drop_meta() {
			Node::Key(left, _, right) => has_splice(left) || has_splice(right),
			Node::List(items, _, _) => items.iter().any(has_splice),
			_ => false,
		}
}

/// The value a hole `$a` or `$(expr)` splices in
fn spliced(code: &Node) -> Option<Node> {
	if let Some(name) = hole_name(code) {
		return Some(Node::Symbol(name.to_string()));
	}
	match code.drop_meta() {
		Node::List(items, Bracket::Round, Separator::None) => match items.as_slice() {
			[dollar, expression] if dollar.is_symbol(SPLICE_CALL) => Some(expression.clone()),
			_ => None,
		},
		_ => None,
	}
}

/// The code that builds the template: parts without holes stay quoted
fn built(code: &Node) -> Node {
	if let Some(value) = spliced(code) {
		return value;
	}
	if !has_splice(code) {
		return quote(code.clone());
	}
	match code.drop_meta() {
		Node::Key(left, op, right) => call(KEY_BUILDER, vec![built(left), text(op.as_str()), built(right)]),
		Node::List(items, bracket, separator) => {
			// `$f($x)`: the call of the code f, as a call of a name is quoted
			let (items, bracket, separator) = match hole_call(items) {
				Some(called) => (called, &Bracket::Round, &Separator::None),
				None => (items.clone(), bracket, separator),
			};
			let opening = match bracket {
				Bracket::None => String::new(),
				written => written.opening().to_string(),
			};
			let bracketed = call(BRACKETED_BUILDER, vec![built_items(&items), text(&opening)]);
			let separator = separator.to_char().map(String::from).unwrap_or_default();
			call(SEPARATED_BUILDER, vec![bracketed, text(&separator)])
		}
		other => quote(other.clone()),
	}
}

/// The items as a list, a rest hole's items joined in: `[a] + xs + [c]`
fn built_items(items: &[Node]) -> Node {
	let mut parts: Vec<Node> = vec![];
	let mut run: Vec<Node> = vec![];
	for item in items {
		match rest_hole(item) {
			Some(name) => {
				parts.extend(listed(std::mem::take(&mut run)));
				parts.push(Node::Symbol(name.to_string()));
			}
			None => run.push(built(item)),
		}
	}
	parts.extend(listed(run));
	let joined = parts.into_iter().reduce(|joined, part| Node::Key(Box::new(joined), Op::Add, Box::new(part)));
	joined.unwrap_or_else(|| Node::List(vec![], Bracket::Square, Separator::Comma))
}

fn listed(items: Vec<Node>) -> Option<Node> {
	(!items.is_empty()).then(|| Node::List(items, Bracket::Square, Separator::Comma))
}
