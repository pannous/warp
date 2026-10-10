//! Ruby blocks (card functions-ruby-yield): `yield v` in a function calls the block its caller passes after the
//! arguments, `each_twice { |x| total += x }`, `twice(3) { |x| … }`, `apply(3) do |x| … end`. Such a function gets the
//! block as its last parameter, `yield a, b` becomes `yield·block(a, b)`. Only functions some call passes a block to
//! are rewritten: `yield` stays free for generators.
//! A block after a call that leaves a parameter of the function without its value is that argument too (Kotlin's and
//! Swift's trailing closure, card web-components): `Card("Hi") { p:"text" }` passes the children, `apply(3) { it*2 }`
//! the function.

use super::nodes::key;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

const YIELD_WORD: &str = "yield";
const BLOCK_PARAMETER: &str = "yield·block";

/// What a function does with a block after its call
enum BlockTaker {
	/// `yield` in its body: the block becomes its extra last parameter
	Yielding,
	/// its parameters, by count: the block is the first one the call leaves without a value
	Parameters(usize),
}

pub fn lower(node: Node) -> Node {
	let takers = block_takers(&node);
	if takers.is_empty() {
		return node;
	}
	let mut called_with_block = HashSet::new();
	let node = with_block_arguments(node, &takers, &mut called_with_block);
	if called_with_block.is_empty() {
		return node;
	}
	with_block_parameters(node, &called_with_block)
}

pub(crate) fn is_yield(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if word == YIELD_WORD)
}

/// `yield`, `yield v` or `yield a, b`: the values yielded
pub(crate) fn yielded(node: &Node) -> Option<Vec<Node>> {
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

pub(crate) fn contains_yield(body: &Node) -> bool {
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

/// The functions a block after a call can go to: those that yield, and those with parameters
fn block_takers(node: &Node) -> HashMap<String, BlockTaker> {
	let mut takers = HashMap::new();
	node.visit(&mut |part| if let Some((name, count)) = parameter_count(part) { takers.insert(name, BlockTaker::Parameters(count)); });
	node.visit(&mut |part| if let Some(name) = definition_name(part) { takers.insert(name, BlockTaker::Yielding); });
	takers
}

/// `f(a, b) := body`: the name and the number of parameters
fn parameter_count(node: &Node) -> Option<(String, usize)> {
	let Node::Key(head, Op::Define | Op::Assign, _) = node.drop_meta() else { return None };
	let Node::List(items, Bracket::Round, _) = head.drop_meta() else { return None };
	let Node::Symbol(name) = items.first()?.drop_meta() else { return None };
	(items.len() > 1).then(|| (name.clone(), items.len() - 1))
}

fn is_block(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(_, Bracket::Curly, _))
}

/// `f`, `f(a)` or `f a`: the function and its arguments
fn call_parts(node: &Node, takers: &HashMap<String, BlockTaker>) -> Option<(String, Vec<Node>)> {
	match node.drop_meta() {
		Node::Symbol(name) if takers.contains_key(name) => Some((name.clone(), vec![])),
		Node::List(items, Bracket::Round | Bracket::None, _) => match items.first()?.drop_meta() {
			Node::Symbol(name) if takers.contains_key(name) => Some((name.clone(), items[1..].iter().flat_map(arguments).collect())),
			_ => None,
		},
		_ => None,
	}
}

/// `(3)` and `(a, b)` hold the arguments
pub(crate) fn arguments(node: &Node) -> Vec<Node> {
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
		false => key(Node::Empty, Op::FatArrow, block),
	}
}

/// The call with the block as its last argument, if the function takes it there
fn call_with_block(name: &str, mut arguments: Vec<Node>, block: Node, takers: &HashMap<String, BlockTaker>) -> Option<Node> {
	let block = match takers.get(name)? {
		BlockTaker::Yielding => as_lambda(block),
		BlockTaker::Parameters(count) if arguments.len() < *count => block,
		BlockTaker::Parameters(_) => return None,
	};
	arguments.insert(0, Node::Symbol(name.to_string()));
	arguments.push(block);
	Some(Node::List(arguments, Bracket::Round, Separator::None))
}

/// `f {…}`, `f(a) {…}`, `f(a) do … end`, `f a do … end`: the block becomes the last argument
fn block_call(node: &Node, takers: &HashMap<String, BlockTaker>) -> Option<(String, Node)> {
	let (callee, block) = match node.drop_meta() {
		Node::List(items, _, Separator::Space | Separator::None) if items.len() == 2 && is_block(&items[1]) => (&items[0], &items[1]),
		Node::Key(callee, Op::Do, block) if is_block(block) => (callee.as_ref(), block.as_ref()),
		Node::List(items, _, Separator::Space) if items.len() == 2 => match items[1].drop_meta() {
			Node::Key(argument, Op::Do, block) if is_block(block) => {
				let (name, mut arguments) = call_parts(&items[0], takers)?;
				arguments.extend(self::arguments(argument));
				return Some((name.clone(), call_with_block(&name, arguments, block.as_ref().clone(), takers)?));
			}
			_ => return None,
		},
		_ => return None,
	};
	let (name, arguments) = call_parts(callee, takers)?;
	Some((name.clone(), call_with_block(&name, arguments, block.clone(), takers)?))
}

fn with_block_arguments(node: Node, takers: &HashMap<String, BlockTaker>, called: &mut HashSet<String>) -> Node {
	if let Some((name, call)) = block_call(&node, takers) {
		if matches!(takers.get(&name), Some(BlockTaker::Yielding)) {
			called.insert(name);
		}
		return call.map_children(|child| with_block_arguments(child, takers, called));
	}
	node.map_children(|child| with_block_arguments(child, takers, called))
}

fn with_block_parameters(node: Node, called: &HashSet<String>) -> Node {
	match node {
		Node::Key(head, Op::Define, body) if definition_name(&Node::Key(head.clone(), Op::Define, body.clone())).is_some_and(|name| called.contains(&name)) => {
			let Node::List(mut items, bracket, separator) = head.drop_meta().clone() else { unreachable!("a function head") };
			items.push(Node::Symbol(BLOCK_PARAMETER.to_string()));
			key(Node::List(items, bracket, separator), Op::Define, block_calls(*body))
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
