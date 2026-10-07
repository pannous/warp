//! Card natural-phrases (samples/natural.wasp), undoable defaults: the list phrases of a method chain.
//! `xs.keep only positive` is `xs where it > 0` (positive, negative, even, odd, or a function of the program),
//! `xs.sort by size` sorts by the key, a function (the program's, a counting word) or else a field (`sort by price`),
//! `xs.take first 10` is `xs.slice(0, 10)`. The parser reads a chain `xs.keep only positive.sort by size` as one list
//! `[xs.keep, only, positive.sort, by, size]`: each `.method` binds to the word before it.

use crate::analyzer::extract_user_functions;
use crate::context::Context;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const KEEP: (&str, &str) = ("keep", "only");
const SORT: (&str, &str) = ("sort", "by");
const TAKE: (&str, &str) = ("take", "first");
/// The conditions `keep only` knows by name, on each element `it`
const PROPERTIES: [(&str, &str); 4] = [("positive", "it > 0"), ("negative", "it < 0"), ("even", "it % 2 == 0"), ("odd", "it % 2 != 0")];
const WHERE_WORD: &str = "where";
const SORT_BY: &str = "sort_by";
const SLICE: &str = "slice";
/// The parameter of a `sort by` key
const ELEMENT: &str = "phrase·element";

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&[KEEP.1, SORT.1, TAKE.1]) {
		return node;
	}
	let mut context = Context::new();
	extract_user_functions(&mut context, &node);
	expand(node, &context)
}

fn expand(node: Node, context: &Context) -> Node {
	let node = node.map_children(|child| expand(child, context));
	match node.drop_meta() {
		Node::List(items, _, _) => chain(items, context).unwrap_or(node),
		_ => node,
	}
}

/// A word of a chain: a value or `.method`
enum Token {
	Value(Node),
	Method(String),
}

/// `[xs.keep, only, positive.sort, by, size]`: the chain applied to its receiver, when every method with words is a
/// phrase; `[ys = xs.keep, only, positive]` assigns the whole chain
fn chain(items: &[Node], context: &Context) -> Option<Node> {
	if let Some(Node::Key(target, op @ (Op::Assign | Op::Define), value)) = items.first().map(Node::drop_meta) {
		let chained = chain(&[vec![value.as_ref().clone()], items[1..].to_vec()].concat(), context)?;
		return Some(Node::Key(target.clone(), *op, Box::new(chained)));
	}
	let mut tokens = vec![];
	for item in items {
		match item.drop_meta() {
			Node::Key(left, Op::Dot, method) if matches!(method.drop_meta(), Node::Symbol(_)) => {
				tokens.push(Token::Value(left.as_ref().clone()));
				tokens.push(Token::Method(method.drop_meta().name()));
			}
			_ => tokens.push(Token::Value(item.clone())),
		}
	}
	let mut tokens = tokens.into_iter().peekable();
	let Some(Token::Value(mut receiver)) = tokens.next() else { return None };
	let mut phrased = false;
	while let Some(token) = tokens.next() {
		let Token::Method(method) = token else { return None };
		let mut words = vec![];
		while let Some(Token::Value(_)) = tokens.peek() {
			let Some(Token::Value(word)) = tokens.next() else { unreachable!() };
			words.push(word);
		}
		receiver = match words.as_slice() {
			[] => Node::Key(Box::new(receiver), Op::Dot, Box::new(Node::Symbol(method))),
			[word, argument] => phrase(&method, &word.drop_meta().name(), receiver, argument, context)?,
			_ => return None,
		};
		phrased |= !words.is_empty();
	}
	phrased.then_some(receiver)
}

fn phrase(method: &str, word: &str, receiver: Node, argument: &Node, context: &Context) -> Option<Node> {
	match (method, word) {
		// parenthesized, so a method after it applies to the filtered list (comprehensions::where_reassociated)
		KEEP => {
			let filter = Node::List(vec![receiver, symbol(WHERE_WORD), condition(argument, context)?], Bracket::None, Separator::Space);
			Some(Node::List(vec![filter], Bracket::Round, Separator::None))
		}
		SORT => Some(method_call(receiver, SORT_BY, vec![key(argument, context)?])),
		TAKE => Some(method_call(receiver, SLICE, vec![Node::Number(crate::extensions::numbers::Number::Int(0)), argument.clone()])),
		_ => None,
	}
}

/// The condition `keep only <argument>` names, on each element `it`: a known property or a function of the program
fn condition(argument: &Node, context: &Context) -> Option<Node> {
	let Node::Symbol(name) = argument.drop_meta() else { return None };
	if let Some((_, condition)) = PROPERTIES.iter().find(|(property, _)| property == name) {
		return Some(crate::wasp_parser::parse(condition));
	}
	context.user_functions.contains_key(name).then(|| call(name, symbol(crate::lambdas::IMPLICIT_PARAMETER)))
}

/// The key `sort by <argument>` names: `element => f(element)` of a function, else `element => element.field`
fn key(argument: &Node, context: &Context) -> Option<Node> {
	let Node::Symbol(name) = argument.drop_meta() else { return None };
	let element = symbol(ELEMENT);
	let is_function = context.user_functions.contains_key(name) || crate::analyzer::is_counting_word(name);
	let value = match is_function {
		true => call(name, element.clone()),
		false => Node::Key(Box::new(element.clone()), Op::Dot, Box::new(argument.clone())),
	};
	Some(Node::Key(Box::new(element), Op::Arrow, Box::new(value)))
}

fn method_call(receiver: Node, method: &str, arguments: Vec<Node>) -> Node {
	let call = Node::List([vec![symbol(method)], arguments].concat(), Bracket::Round, Separator::None);
	Node::Key(Box::new(receiver), Op::Dot, Box::new(call))
}

fn call(name: &str, argument: Node) -> Node {
	Node::List(vec![symbol(name), argument], Bracket::Round, Separator::None)
}

fn symbol(name: &str) -> Node {
	Node::Symbol(name.to_string())
}
