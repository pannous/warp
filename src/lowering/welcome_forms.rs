//! Forms other languages use, lowered to wasp's own (notes/welcoming.md), before any other pass:
//! - `match v { 0 => "zero"; _ => "other" }`: the cases of switch/match written with `=>`, `_` the default
//! - `loop { … }`: `while true { … }`, left by `break`
//! - anonymous functions `function(a, b) { … }`, `fn(x) { … }` (JS, Rust-ish), `f <- function(x) x * 2` (R) and
//!   `lambda x: …` (Python): lambdas
//!   (`xs |> f(b)` is read by the parser, `f(1, _)` lowered in declarations.rs)
//! - OCaml / F# `let f x = body in rest`: the definition `f(x) := body`, then rest

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const SWITCH_WORDS: [&str; 2] = ["switch", "match"];
const WILDCARD: &str = "_";
const DEFAULT_CASE: &str = "default";
const LOOP_WORD: &str = "loop";
const LAMBDA_WORD: &str = "lambda";
const LET_WORD: &str = "let";
const IN_WORD: &str = "in";

pub fn lower(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = python_lambdas(items.into_iter().map(lower).collect());
			if let Some(lambda) = anonymous_function(&items).or_else(|| assigned_braceless_function(&items)) {
				return lambda;
			}
			if let Some(binding) = let_binding(&items) {
				return binding;
			}
			let items = go_destructuring(items, &separator);
			endless_loop(&items).unwrap_or_else(|| Node::List(arrow_cases(items), bracket, separator))
		}
		Node::Key(left, Op::Colon, body) if lambda_parameters(&left).is_some() => lambda(lambda_parameters(&left).expect("guarded"), lower(*body)),
		Node::Key(left, op, right) => Node::Key(Box::new(lower(*left)), op, Box::new(lower(*right))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
		other => other,
	}
}

/// OCaml / F# `let f x = body in rest`, `let f x = body`, `let x = v in rest`: the definition `f(x) := body` (or the
/// assignment) and then rest
fn let_binding(items: &[Node]) -> Option<Node> {
	let [keyword, names @ .., last] = items else { return None };
	let Node::Key(last_name, Op::Assign, value) = last.drop_meta() else { return None };
	if !is_word(keyword, LET_WORD) {
		return None;
	}
	let names: Vec<&Node> = names.iter().chain([last_name.as_ref()]).collect();
	if !names.iter().all(|name| matches!(name.drop_meta(), Node::Symbol(_))) {
		return None;
	}
	let (body, rest) = split_at_in(value);
	if names.len() == 1 && rest.is_none() {
		return None; // `let x = 5`: a declaration as it is
	}
	let (name, parameters) = names.split_first().expect("a name");
	let binding = match parameters.is_empty() {
		true => Node::Key(Box::new((*name).clone()), Op::Assign, Box::new(lower(body))),
		false => {
			let head = Node::List(names.iter().map(|name| (*name).clone()).collect(), Bracket::Round, Separator::None);
			Node::Key(Box::new(head), Op::Define, Box::new(lower(body)))
		}
	};
	Some(match rest {
		Some(rest) => Node::List(vec![binding, lower(rest)], Bracket::Round, Separator::Semicolon),
		None => binding,
	})
}

/// Go's `q, r := f(x)`: names taking the values apart, `q, r = f(x)` (`r := …` alone would define a getter)
fn go_destructuring(mut items: Vec<Node>, separator: &Separator) -> Vec<Node> {
	let Some(position) = items.iter().position(|item| matches!(item.drop_meta(), Node::Key(_, Op::Define, _))) else { return items };
	let is_name = |node: &Node| matches!(node.drop_meta(), Node::Symbol(_));
	let Node::Key(name, Op::Define, value) = items[position].drop_meta() else { return items };
	if *separator != Separator::Colon || position == 0 || !is_name(name) || !items[..position].iter().all(is_name) {
		return items;
	}
	items[position] = Node::Key(name.clone(), Op::Assign, value.clone());
	items
}

/// `x * 2 in double 4` → (`x * 2`, `double 4`); no `in`: the value alone
fn split_at_in(value: &Node) -> (Node, Option<Node>) {
	let Node::List(words, Bracket::None, separator @ (Separator::Space | Separator::None)) = value.drop_meta() else { return (value.clone(), None) };
	let Some(position) = words.iter().position(|word| is_word(word, IN_WORD)) else { return (value.clone(), None) };
	let phrase = |part: &[Node]| match part {
		[single] => single.clone(),
		several => Node::List(several.to_vec(), Bracket::None, separator.clone()),
	};
	(phrase(&words[..position]), Some(phrase(&words[position + 1..])))
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
	if !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	Some(lambda(function_parameters(head)?, lower(body.clone())))
}

/// R's `f <- function(x) x * 2`, JS-ish `f = function(x) x * 2`: the assignment takes the phrase after the head as
/// the body, `f = x => x * 2`
fn assigned_braceless_function(items: &[Node]) -> Option<Node> {
	let [assignment, body @ ..] = items else { return None };
	let Node::Key(name, Op::Assign, head) = assignment.drop_meta() else { return None };
	let body = match body {
		[] => return None,
		[single] => single.clone(),
		several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
	};
	Some(Node::Key(name.clone(), Op::Assign, Box::new(lambda(function_parameters(head)?, lower(body)))))
}

/// `function (a, b)`: the parameters after a function keyword with no name
fn function_parameters(head: &Node) -> Option<Node> {
	let Node::List(head_items, _, _) = head.drop_meta() else { return None };
	let [keyword, parameters] = head_items.as_slice() else { return None };
	let is_keyword = matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word));
	is_keyword.then(|| parameters.clone())
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
