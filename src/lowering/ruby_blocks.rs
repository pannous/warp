//! Ruby blocks (card functions-ruby-yield): `yield v` in a function calls the block its caller passes after the
//! arguments, `each_twice { |x| total += x }`, `twice(3) { |x| … }`, `apply(3) do |x| … end`. Such a function gets the
//! block as its last parameter, `yield a, b` becomes `yield·block(a, b)`. Only functions some call passes a block to
//! are rewritten: `yield` stays free for generators.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;

const YIELD_WORD: &str = "yield";
const BLOCK_PARAMETER: &str = "yield·block";

pub fn lower(node: Node) -> Node {
	let yielding = yielding_functions(&node);
	if yielding.is_empty() {
		return node;
	}
	let mut called_with_block = HashSet::new();
	let node = with_block_arguments(node, &yielding, &mut called_with_block);
	if called_with_block.is_empty() {
		return node;
	}
	with_block_parameters(node, &called_with_block)
}

fn is_yield(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if word == YIELD_WORD)
}

/// `yield`, `yield v` or `yield a, b`: the values yielded
fn yielded(node: &Node) -> Option<Vec<Node>> {
	match node.drop_meta() {
		_ if is_yield(node) => Some(vec![]),
		Node::List(items, _, Separator::Space | Separator::None) if items.first().is_some_and(is_yield) => Some(items[1..].to_vec()),
		Node::List(items, _, Separator::Colon) => {
			let mut values = yielded(items.first()?)?;
			values.extend(items[1..].iter().cloned());
			Some(values)
		}
		_ => None,
	}
}

fn contains_yield(body: &Node) -> bool {
	let mut found = false;
	body.visit(&mut |part| found |= is_yield(part));
	found
}

/// `f(params) := body` with a head of the function's name
fn definition_name(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Key(head, Op::Define, body) if contains_yield(body) => match head.drop_meta() {
			Node::List(items, Bracket::Round, _) => match items.first()?.drop_meta() {
				Node::Symbol(name) => Some(name.clone()),
				_ => None,
			},
			_ => None,
		},
		_ => None,
	}
}

fn yielding_functions(node: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	node.visit(&mut |part| names.extend(definition_name(part)));
	names
}

fn is_block(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(_, Bracket::Curly, _))
}

/// `f`, `f(a)` or `f a`: the function and its arguments
fn call_parts(node: &Node, yielding: &HashSet<String>) -> Option<(String, Vec<Node>)> {
	match node.drop_meta() {
		Node::Symbol(name) if yielding.contains(name) => Some((name.clone(), vec![])),
		Node::List(items, Bracket::Round | Bracket::None, _) => match items.first()?.drop_meta() {
			Node::Symbol(name) if yielding.contains(name) => Some((name.clone(), items[1..].iter().flat_map(arguments).collect())),
			_ => None,
		},
		_ => None,
	}
}

/// `(3)` and `(a, b)` hold the arguments
fn arguments(node: &Node) -> Vec<Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, Separator::Colon) => items.clone(),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => items.clone(),
		_ => vec![node.clone()],
	}
}

/// `{ |x| … }` is a lambda already, `{ 5 }` becomes `() => { 5 }`
fn as_lambda(block: Node) -> Node {
	match crate::lambdas::arrow_lambda(&block).is_some() || crate::lambdas::block_as_arrow(&block).is_some() {
		true => block,
		false => Node::Key(Box::new(Node::Empty), Op::FatArrow, Box::new(block)),
	}
}

fn call_with_block(name: String, mut arguments: Vec<Node>, block: Node) -> Node {
	arguments.insert(0, Node::Symbol(name));
	arguments.push(as_lambda(block));
	Node::List(arguments, Bracket::Round, Separator::None)
}

/// `f {…}`, `f(a) {…}`, `f(a) do … end`, `f a do … end`: the block becomes the last argument
fn block_call(node: &Node, yielding: &HashSet<String>) -> Option<(String, Node)> {
	let (callee, block) = match node.drop_meta() {
		Node::List(items, _, Separator::Space | Separator::None) if items.len() == 2 && is_block(&items[1]) => (&items[0], &items[1]),
		Node::Key(callee, Op::Do, block) if is_block(block) => (callee.as_ref(), block.as_ref()),
		Node::List(items, _, Separator::Space) if items.len() == 2 => match items[1].drop_meta() {
			Node::Key(argument, Op::Do, block) if is_block(block) => {
				let (name, mut arguments) = call_parts(&items[0], yielding)?;
				arguments.extend(self::arguments(argument));
				return Some((name.clone(), call_with_block(name, arguments, block.as_ref().clone())));
			}
			_ => return None,
		},
		_ => return None,
	};
	let (name, arguments) = call_parts(callee, yielding)?;
	Some((name.clone(), call_with_block(name, arguments, block.clone())))
}

fn with_block_arguments(node: Node, yielding: &HashSet<String>, called: &mut HashSet<String>) -> Node {
	if let Some((name, call)) = block_call(&node, yielding) {
		called.insert(name);
		return call.map_children(|child| with_block_arguments(child, yielding, called));
	}
	node.map_children(|child| with_block_arguments(child, yielding, called))
}

fn with_block_parameters(node: Node, called: &HashSet<String>) -> Node {
	match node {
		Node::Key(head, Op::Define, body) if definition_name(&Node::Key(head.clone(), Op::Define, body.clone())).is_some_and(|name| called.contains(&name)) => {
			let Node::List(mut items, bracket, separator) = head.drop_meta().clone() else { unreachable!("a function head") };
			items.push(Node::Symbol(BLOCK_PARAMETER.to_string()));
			Node::Key(Box::new(Node::List(items, bracket, separator)), Op::Define, Box::new(block_calls(*body)))
		}
		other => other.map_children(|child| with_block_parameters(child, called)),
	}
}

fn block_calls(node: Node) -> Node {
	match yielded(&node) {
		Some(values) => {
			let callee = std::iter::once(Node::Symbol(BLOCK_PARAMETER.to_string()));
			Node::List(callee.chain(values.into_iter().map(block_calls)).collect(), Bracket::Round, Separator::None)
		}
		None => node.map_children(block_calls),
	}
}
