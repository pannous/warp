//! Card natural-phrases (samples/natural.warp), undoable defaults: the list phrases of a method chain.
//! `xs.keep only positive` is `xs where it > 0` (positive, negative, even, odd, or a function of the program),
//! `xs.sort by size` sorts by the key, a function (the program's, a counting word) or else a field (`sort by price`),
//! `xs.take first 10` is `xs.slice(0, 10)`. The parser reads a chain `xs.keep only positive.sort by size` as one list
//! `[xs.keep, only, positive.sort, by, size]`: each `.method` binds to the word before it.

use super::nodes::{call, key, symbol};
use crate::context::Context;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const KEEP: (&str, &str) = ("keep", "only");
const SORT: (&str, &str) = ("sort", "by");
const TAKE: (&str, &str) = ("take", "first");
const PHRASES: [(&str, &str); 3] = [KEEP, SORT, TAKE];
/// Other spellings of a phrase's method: `todos sorted by priority`
const METHOD_ALIASES: [(&str, &str); 1] = [("sorted", "sort")];
/// The conditions `keep only` knows by name, on each element `it`
const PROPERTIES: [(&str, &str); 4] = [("positive", "it > 0"), ("negative", "it < 0"), ("even", "it % 2 == 0"), ("odd", "it % 2 != 0")];
const WHERE_KEYWORD: &str = "where";
const SORT_BY: &str = "sort_by";
const SLICE: &str = "slice";
/// The parameter of a `sort by` key
const ELEMENT: &str = "phrase·element";

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&[KEEP.1, SORT.1, TAKE.1]) {
		return node;
	}
	let context = crate::analyzer::function_context(&node);
	expand(node, &context)
}

fn expand(node: Node, context: &Context) -> Node {
	let node = node.map_children(|child| expand(child, context));
	match node.drop_meta() {
		Node::List(items, _, _) => sorted_by(items, context).or_else(|| chain(items, context)).unwrap_or(node),
		_ => node,
	}
}

/// `xs sorted by it.age`, `xs sorted by -it.age` (read as `by - it.age`), `xs sort by name`: `xs.sort_by(it => key)`
fn sorted_by(items: &[Node], context: &Context) -> Option<Node> {
	let flat = words(items);
	let items = flat.as_slice();
	let is_word = |node: &Node, words: &[&str]| matches!(node.drop_meta(), Node::Symbol(word) if words.contains(&canonical(word).as_str()));
	// `xs.sort by -it` parses as `[xs.sort, by - it]`
	if let [Node::Key(receiver, Op::Dot, sort), descending] = items.iter().map(Node::drop_meta).collect::<Vec<_>>().as_slice() {
		if is_word(sort, &[SORT.0]) {
			return sorted_by(&[receiver.as_ref().clone(), sort.as_ref().clone(), (*descending).clone()], context);
		}
	}
	let (receiver, key_written) = match items {
		[receiver, sort, by, key] if is_word(sort, &[SORT.0]) && is_word(by, &[SORT.1]) => (receiver, key.clone()),
		[receiver, sort, descending] if is_word(sort, &[SORT.0]) => match descending.drop_meta() {
			Node::Key(by, Op::Sub, key) if is_word(by, &[SORT.1]) => (receiver, Node::Key(Box::new(Node::Empty), Op::Neg, key.clone())),
			_ => return None,
		},
		_ => return None,
	};
	Some(method_call(receiver.clone(), SORT_BY, vec![sort_key(&key_written, context)?]))
}

/// The key of `sort by it.age` as `it => it.age`, of `sort by size` as key names it
fn sort_key(written: &Node, context: &Context) -> Option<Node> {
	let it = crate::lambdas::IMPLICIT_PARAMETER;
	match crate::lambdas::mentions(written, it) {
		true => Some(key(symbol(it), Op::Arrow, written.clone())),
		false => key_function(written, context),
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
	// `str(xs.take first 3)` parses as the call `[str, xs.take, first, 3]`: the chain is its argument
	if let [callee, argument @ ..] = items {
		if let (Node::Symbol(function), true) = (callee.drop_meta(), argument.len() > 1) {
			if let Some(chained) = chain(argument, context) {
				return Some(call(function, vec![chained]));
			}
		}
	}
	let items = words(items);
	let mut tokens = vec![];
	for (index, item) in items.iter().enumerate() {
		match item.drop_meta() {
			Node::Key(left, Op::Dot, method) if matches!(method.drop_meta(), Node::Symbol(_)) => {
				tokens.push(Token::Value(left.as_ref().clone()));
				tokens.push(Token::Method(canonical(&method.drop_meta().name())));
			}
			// `xs sorted by price`: the method word without its dot, when its phrase word follows
			Node::Symbol(word) if index > 0 && starts_phrase(&canonical(word), items.get(index + 1)) => tokens.push(Token::Method(canonical(word))),
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
			[] => key(receiver, Op::Dot, Node::Symbol(method)),
			[word, argument] => phrase(&method, &word.drop_meta().name(), receiver, argument, context)?,
			_ => return None,
		};
		phrased |= !words.is_empty();
	}
	phrased.then_some(receiver)
}

/// The words of a phrase, flat: `cheapest = items sorted by price` reads as `[[items, [sorted, by]], price]`
pub(crate) fn words(items: &[Node]) -> Vec<Node> {
	items.iter().flat_map(|item| match item.drop_meta() {
		Node::List(inner, Bracket::None, Separator::Space) => words(inner),
		_ => vec![item.clone()],
	}).collect()
}

fn canonical(method: &str) -> String {
	METHOD_ALIASES.iter().find(|(alias, _)| *alias == method).map_or(method, |(_, name)| name).to_string()
}

fn starts_phrase(method: &str, next: Option<&Node>) -> bool {
	let Some(Node::Symbol(word)) = next.map(Node::drop_meta) else { return false };
	PHRASES.contains(&(method, word.as_str()))
}

fn phrase(method: &str, word: &str, receiver: Node, argument: &Node, context: &Context) -> Option<Node> {
	match (method, word) {
		// parenthesized, so a method after it applies to the filtered list (comprehensions::where_reassociated)
		KEEP => {
			let filter = Node::List(vec![receiver, symbol(WHERE_KEYWORD), condition(argument, context)?], Bracket::None, Separator::Space);
			Some(Node::List(vec![filter], Bracket::Round, Separator::None))
		}
		SORT => Some(method_call(receiver, SORT_BY, vec![sort_key(argument, context)?])),
		TAKE => Some(method_call(receiver, SLICE, vec![Node::Number(crate::extensions::numbers::Number::Int(0)), argument.clone()])),
		_ => None,
	}
}

/// The condition `keep only <argument>` names, on each element `it`: a known property or a function of the program
fn condition(argument: &Node, context: &Context) -> Option<Node> {
	let Node::Symbol(name) = argument.drop_meta() else { return None };
	if let Some((_, condition)) = PROPERTIES.iter().find(|(property, _)| property == name) {
		return Some(crate::warp_parser::parse(condition));
	}
	context.user_functions.contains_key(name).then(|| call(name, vec![symbol(crate::lambdas::IMPLICIT_PARAMETER)]))
}

/// The key `sort by <argument>` names: `element => f(element)` of a function, else `element => element.field`
fn key_function(argument: &Node, context: &Context) -> Option<Node> {
	let element = symbol(ELEMENT);
	// `sort by abs`: a function word the parser reads as its prefix operator, without an operand
	if let Node::Key(left, op, right) = argument.drop_meta() {
		let applied = Node::Key(left.clone(), *op, Box::new(element.clone()));
		return matches!((left.drop_meta(), right.drop_meta()), (Node::Empty, Node::Empty)).then(|| key(element, Op::Arrow, applied));
	}
	let Node::Symbol(name) = argument.drop_meta() else { return None };
	let is_function = context.user_functions.contains_key(name) || crate::analyzer::is_counting_word(name);
	let value = match is_function {
		true => call(name, vec![element.clone()]),
		false => key(element.clone(), Op::Dot, argument.clone()),
	};
	Some(key(element, Op::Arrow, value))
}

fn method_call(receiver: Node, method: &str, arguments: Vec<Node>) -> Node {
	let call = Node::List([vec![symbol(method)], arguments].concat(), Bracket::Round, Separator::None);
	key(receiver, Op::Dot, call)
}

