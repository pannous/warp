//! Forms other languages use, lowered to wasp's own (notes/welcoming.md), before any other pass:
//! - `match v { 0 => "zero"; _ => "other" }`: the cases of switch/match written with `=>`, `_` the default
//! - `loop { … }`: `while true { … }`, left by `break`
//! - anonymous functions `function(a, b) { … }`, `fn(x) { … }` (JS, Rust-ish), `f <- function(x) x * 2` (R) and
//!   `lambda x: …` (Python): lambdas
//!   (`xs |> f(b)` is read by the parser, `f(1, _)` lowered in declarations.rs)
//! - OCaml / F# `let f x = body in rest`: the definition `f(x) := body`, then rest
//! - JS destructured parameters `({a, b}) => a + b`: the object taken apart into its fields

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;

const SWITCH_WORDS: [&str; 2] = ["switch", "match"];
const WILDCARD: &str = "_";
const DEFAULT_CASE: &str = "default";
const LOOP_WORD: &str = "loop";
const LAMBDA_WORD: &str = "lambda";
const LET_WORD: &str = "let";
const IN_WORD: &str = "in";
const DESTRUCTURED_OBJECT: &str = "object·";
const RUBY_LAMBDA_WORDS: [&str; 2] = ["lambda", "proc"];
const CALL_METHOD: &str = "call";

pub fn lower(node: Node) -> Node {
	let node = forms(node);
	let mut lambda_names = HashSet::new();
	collect_lambda_names(&node, &mut lambda_names);
	lambda_calls(node, &lambda_names)
}

/// The names assigned a lambda: `f = x => …`, `f = { |x| … }`
fn collect_lambda_names(node: &Node, names: &mut HashSet<String>) {
	match node.drop_meta() {
		Node::Key(name, Op::Assign, value) if matches!(name.drop_meta(), Node::Symbol(_)) && crate::lambdas::arrow_lambda(value).is_some() => {
			names.insert(name.name());
		}
		Node::Key(left, _, right) => [left, right].into_iter().for_each(|side| collect_lambda_names(side, names)),
		Node::List(items, _, _) => items.iter().for_each(|item| collect_lambda_names(item, names)),
		_ => {}
	}
}

/// Ruby's `f.call(3)` of a lambda f: the call `f(3)`; a method `call` of an object stays
fn lambda_calls(node: Node, lambda_names: &HashSet<String>) -> Node {
	match node {
		Node::Key(function, Op::Dot, call) if lambda_names.contains(&function.name()) && called_arguments(&call).is_some() => {
			let arguments = called_arguments(&call).expect("guarded").into_iter().map(|argument| lambda_calls(argument, lambda_names));
			Node::List([*function].into_iter().chain(arguments).collect(), Bracket::Round, Separator::None)
		}
		other => other.map_children(|child| lambda_calls(child, lambda_names)),
	}
}

fn forms(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = python_lambdas(items.into_iter().map(forms).collect());
			if let Some(lambda) = anonymous_function(&items).or_else(|| assigned_braceless_function(&items)).or_else(|| ruby_lambda(&items)) {
				return lambda;
			}
			if let Some(binding) = let_binding(&items) {
				return binding;
			}
			let items = go_destructuring(items, &separator);
			endless_loop(&items).unwrap_or_else(|| Node::List(arrow_cases(items), bracket, separator))
		}
		Node::Key(left, Op::Colon, body) if lambda_parameters(&left).is_some() => lambda(lambda_parameters(&left).expect("guarded"), forms(*body)),
		Node::Key(parameters, Op::FatArrow, body) if destructured_parameters(&parameters).is_some() => {
			let (parameters, fields) = destructured_parameters(&parameters).expect("guarded");
			lambda(parameters, Node::List([fields, vec![forms(*body)]].concat(), Bracket::Curly, Separator::Semicolon))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(forms(*left)), op, Box::new(forms(*right))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(forms(*node)), data },
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
		true => Node::Key(Box::new((*name).clone()), Op::Assign, Box::new(forms(body))),
		false => {
			let head = Node::List(names.iter().map(|name| (*name).clone()).collect(), Bracket::Round, Separator::None);
			Node::Key(Box::new(head), Op::Define, Box::new(forms(body)))
		}
	};
	Some(match rest {
		Some(rest) => Node::List(vec![binding, forms(rest)], Bracket::Round, Separator::Semicolon),
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
	Some(lambda(function_parameters(head)?, forms(body.clone())))
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
	Some(Node::Key(name.clone(), Op::Assign, Box::new(lambda(function_parameters(head)?, forms(body)))))
}

/// JS `({a, b}, k) => …`: an object parameter taken apart, `(object·0, k) => { a = object·0.a; b = object·0.b; … }`
fn destructured_parameters(parameters: &Node) -> Option<(Node, Vec<Node>)> {
	let (items, separator) = match parameters.drop_meta() {
		Node::List(items, Bracket::Round, separator) => (items.clone(), separator.clone()),
		single => (vec![single.clone()], Separator::Colon),
	};
	let mut fields = Vec::new();
	let parameters: Vec<Node> = items.into_iter().enumerate().map(|(index, parameter)| match parameter.drop_meta() {
		Node::List(names, Bracket::Curly, _) if !names.is_empty() && names.iter().all(|name| matches!(name.drop_meta(), Node::Symbol(_))) => {
			let object = Node::Symbol(format!("{DESTRUCTURED_OBJECT}{index}"));
			let field = |name: &Node| Node::Key(Box::new(name.clone()), Op::Assign, Box::new(Node::Key(Box::new(object.clone()), Op::Dot, Box::new(name.clone()))));
			fields.extend(names.iter().map(field));
			object
		}
		_ => parameter,
	}).collect();
	(!fields.is_empty()).then(|| (Node::List(parameters, Bracket::Round, separator), fields))
}

/// Ruby's `lambda { |x| x * x }`, `proc { |x| … }`: the block, a lambda itself
fn ruby_lambda(items: &[Node]) -> Option<Node> {
	let [word, block] = items else { return None };
	let is_lambda_word = RUBY_LAMBDA_WORDS.iter().any(|lambda_word| is_word(word, lambda_word));
	(is_lambda_word && matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _))).then(|| block.clone())
}

/// `call(3)`, `call 3`: the arguments of Ruby's `.call`
fn called_arguments(call: &Node) -> Option<Vec<Node>> {
	let Node::List(items, _, _) = call.drop_meta() else { return is_word(call, CALL_METHOD).then(Vec::new) };
	let [word, arguments @ ..] = items.as_slice() else { return None };
	is_word(word, CALL_METHOD).then(|| arguments.to_vec())
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
