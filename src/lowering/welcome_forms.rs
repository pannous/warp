//! Forms other languages use, lowered to wasp's own (notes/welcoming.md), before any other pass:
//! - `match v { 0 => "zero"; _ => "other" }`: the cases of switch/match written with `=>`, `_` the default
//! - `loop { … }`: `while true { … }`, left by `break`
//! - anonymous functions `function(a, b) { … }`, `fn(x) { … }` (JS, Rust-ish) and `lambda x: …` (Python): lambdas
//!   (`xs |> f(b)` is read by the parser, `f(1, _)` lowered in declarations.rs)

use crate::node::{Bracket, Node};
use crate::operators::Op;

const SWITCH_WORDS: [&str; 2] = ["switch", "match"];
const WILDCARD: &str = "_";
const DEFAULT_CASE: &str = "default";
const LOOP_WORD: &str = "loop";
const LAMBDA_WORD: &str = "lambda";

pub fn lower(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = python_lambdas(items.into_iter().map(lower).collect());
			if let Some(lambda) = anonymous_function(&items) {
				return lambda;
			}
			endless_loop(&items).unwrap_or_else(|| Node::List(arrow_cases(items), bracket, separator))
		}
		Node::Key(left, Op::Colon, body) if lambda_parameters(&left).is_some() => lambda(lambda_parameters(&left).expect("guarded"), lower(*body)),
		Node::Key(left, op, right) => Node::Key(Box::new(lower(*left)), op, Box::new(lower(*right))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
		other => other,
	}
}

fn is_word(node: &Node, word: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == word)
}

/// `[match, subject, {k => v …}]`: the cases as `k: v`, `_ => v` as `default: v`
fn arrow_cases(mut items: Vec<Node>) -> Vec<Node> {
	let is_switch = items.len() == 3 && SWITCH_WORDS.iter().any(|word| is_word(&items[0], word));
	if !is_switch {
		return items;
	}
	if let Node::List(cases, Bracket::Curly, separator) = items[2].drop_meta() {
		let cases = cases.iter().map(|case| match case.drop_meta() {
			Node::Key(pattern, Op::FatArrow, body) => {
				let pattern = if is_word(pattern, WILDCARD) { Node::Symbol(DEFAULT_CASE.to_string()) } else { pattern.as_ref().clone() };
				Node::Key(Box::new(pattern), Op::Colon, body.clone())
			}
			_ => case.clone(),
		});
		items[2] = Node::List(cases.collect(), Bracket::Curly, separator.clone());
	}
	items
}

/// `loop { body }`
fn endless_loop(items: &[Node]) -> Option<Node> {
	let [word, body] = items else { return None };
	if !is_word(word, LOOP_WORD) || !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	let condition = Node::Key(Box::new(Node::Empty), Op::While, Box::new(Node::True));
	Some(Node::Key(Box::new(condition), Op::Do, Box::new(body.clone())))
}

fn lambda(parameters: Node, body: Node) -> Node {
	Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body))
}

/// `[function (a, b)] {body}`: a function keyword with parameters and no name, then its body
fn anonymous_function(items: &[Node]) -> Option<Node> {
	let [head, body] = items else { return None };
	let Node::List(head_items, _, _) = head.drop_meta() else { return None };
	let [keyword, parameters @ ..] = head_items.as_slice() else { return None };
	let is_keyword = matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word));
	if !is_keyword || !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	let parameters = match parameters {
		[single] => single.clone(),
		_ => return None,
	};
	Some(lambda(parameters, lower(body.clone())))
}

/// `lambda x` (the words before Python's colon): the parameters
fn lambda_parameters(words: &Node) -> Option<Node> {
	let Node::List(items, Bracket::None, _) = words.drop_meta() else { return None };
	match items.as_slice() {
		[word, parameter] if is_word(word, LAMBDA_WORD) => Some(parameter.clone()),
		_ => None,
	}
}

/// `map xs lambda x: x+1` arrives as the items `…, lambda, x: x+1`: the two are one lambda
fn python_lambdas(items: Vec<Node>) -> Vec<Node> {
	let mut merged: Vec<Node> = Vec::with_capacity(items.len());
	for item in items {
		let follows_lambda = merged.last().is_some_and(|last| is_word(last, LAMBDA_WORD));
		match item.drop_meta() {
			Node::Key(parameters, Op::Colon, body) if follows_lambda => {
				merged.pop();
				merged.push(lambda(parameters.as_ref().clone(), body.as_ref().clone()));
			}
			_ => merged.push(item),
		}
	}
	merged
}
