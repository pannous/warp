//! `for` loops lower to the `while` loop: a range counts its variable up, any other iterable is walked by an index.
//!
//! `for i in a..b body` → `i=a; while i<b {body; i++}` (`...` and `to` include b: `i<=b`), so `i` is b (b+1) afterwards
//! `for x in xs body`   → `items=xs; index=0; while index<#items {x=items#(index+1); body; index++}`
//! `for(init;test;step){body}` → `init; while test {body; step}`

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasm_emitter::mark_step;
use crate::wasp_parser::while_do;

const FOR_KEYWORD: &str = "for";
const IN_KEYWORD: &str = "in";
const IMPLICIT_VARIABLE: &str = "it";
const RANGE_CALL: &str = "range";
const FIRST_INDEX: i64 = 0;

/// The `while` lowering of a `for` loop, or the node itself when it is not one
pub fn lower(node: Node) -> Result<Node, Node> {
	let lowered = match node.drop_meta() {
		Node::List(items, Bracket::Round | Bracket::None, _) => classic_for(items),
		_ => None,
	};
	let lowered = lowered.or_else(|| match node.drop_meta() {
		Node::List(items, _, _) => for_in(items).or_else(|| for_it(items)),
		_ => None,
	});
	lowered.ok_or(node)
}

fn symbol(name: &str) -> Node {
	Node::Symbol(name.to_string())
}

fn number(value: i64) -> Node {
	Node::Number(crate::extensions::numbers::Number::Int(value))
}

fn key(left: Node, op: Op, right: Node) -> Node {
	Node::Key(Box::new(left), op, Box::new(right))
}

fn block(statements: Vec<Node>, bracket: Bracket) -> Node {
	Node::List(statements, bracket, Separator::Semicolon)
}

fn is_word(node: &Node, word: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if name == word)
}

/// The statements of a body: `{a;b}` and `{a⏎b}` hold several, `{x}` and `{for i in xs {…}}` one
pub(crate) fn block_items(body: &Node) -> Vec<Node> {
	match body.drop_meta() {
		Node::List(items, Bracket::Curly, Separator::Semicolon | Separator::Newline) => items.clone(),
		Node::List(items, Bracket::Curly, separator) if items.len() > 1 => vec![Node::List(items.clone(), Bracket::None, separator.clone())],
		Node::List(items, Bracket::Curly, _) => items.clone(),
		single => vec![single.clone()],
	}
}

/// `for(init;test;step){body}`, parsed as `((for (init;test;step)) {body})`
fn classic_for(items: &[Node]) -> Option<Node> {
	let [header, body] = items else { return None };
	let Node::List(header, _, _) = header.drop_meta() else { return None };
	let [keyword, parts] = header.as_slice() else { return None };
	let Node::List(parts, Bracket::Round, Separator::Semicolon) = parts.drop_meta() else { return None };
	let [init, test, step] = parts.as_slice() else { return None };
	if !is_word(keyword, FOR_KEYWORD) || !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	let mut statements = block_items(body);
	statements.push(mark_step(step.clone()));
	Some(block(vec![init.clone(), while_do(test.clone(), block(statements, Bracket::Curly))], Bracket::None))
}

/// `for x in iterable {body}` and `for x in iterable: body`
fn for_in(items: &[Node]) -> Option<Node> {
	let [keyword, variable, in_word, rest @ ..] = items else { return None };
	if !is_word(keyword, FOR_KEYWORD) || !is_word(in_word, IN_KEYWORD) {
		return None;
	}
	let names = loop_names(variable)?;
	let (iterable, body) = match rest {
		[iterable, body] => (iterable.clone(), body.clone()),
		[colon] => match colon.drop_meta() {
			Node::Key(iterable, Op::Colon, body) => (iterable.as_ref().clone(), body.as_ref().clone()),
			_ => return None,
		},
		_ => return None,
	};
	let mut body = block_items(&body);
	// `for word in text: print it`: it is the loop variable too
	if matches!(names.as_slice(), [name] if name != IMPLICIT_VARIABLE) {
		body = body.into_iter().map(|statement| it_as(statement, variable)).collect();
	}
	if names.len() > 1 {
		return Some(destructuring_loop(&names, iterable, body));
	}
	Some(match counting_range(&iterable).drop_meta() {
		Node::Key(start, op @ (Op::Range | Op::To), end) => counting_loop(variable, start, *op, end, body),
		_ => walking_loop(variable, iterable, body),
	})
}

/// `it` replaced by the loop variable, except inside a block or a lambda of the body, whose own parameter it is
fn it_as(node: Node, variable: &Node) -> Node {
	match node {
		Node::Symbol(name) if name == IMPLICIT_VARIABLE => variable.clone(),
		Node::List(_, Bracket::Curly, _) | Node::Key(_, Op::Arrow | Op::FatArrow, _) => node,
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| it_as(item, variable)).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(it_as(*left, variable)), op, Box::new(it_as(*right, variable))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(it_as(*node, variable)), data },
		other => other,
	}
}

/// The range an iterable spells: `(a..b)` is `a..b`, Python's `range(n)` is `0..n` and `range(a, b)` is `a..b`
fn counting_range(iterable: &Node) -> Node {
	match iterable.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => counting_range(&items[0]),
		Node::List(items, _, _) if is_word(&items[0], RANGE_CALL) => match &items[1..] {
			[end] => key(number(FIRST_INDEX), Op::Range, end.clone()),
			[start, end] => key(start.clone(), Op::Range, end.clone()),
			_ => iterable.clone(),
		},
		_ => iterable.clone(),
	}
}

/// The names a loop binds: `x`, or the parts of each element in `for (r, c) in …` and `for k, v in …`
fn loop_names(variable: &Node) -> Option<Vec<Node>> {
	match variable.drop_meta() {
		Node::Symbol(_) => Some(vec![variable.clone()]),
		Node::List(names, Bracket::Round | Bracket::None, _) if !names.is_empty() && names.iter().all(|name| matches!(name.drop_meta(), Node::Symbol(_))) => {
			Some(names.clone())
		}
		_ => None,
	}
}

/// `for (r, c) in pairs {body}` → `for r·c in map_entries(pairs) {r = r·c#1; c = r·c#2; body}`:
/// each name is bound to its part of the element; the elements of a map are its `key:value` entries
fn destructuring_loop(names: &[Node], iterable: Node, body: Vec<Node>) -> Node {
	let element = symbol(&names.iter().map(Node::name).collect::<Vec<_>>().join("·"));
	let entries = Node::List(vec![symbol(crate::library_words::MAP_ENTRIES), iterable], Bracket::Round, Separator::None);
	let parts = names.iter().enumerate().map(|(position, name)| key(name.clone(), Op::Assign, key(element.clone(), Op::Hash, number(position as i64 + 1))));
	walking_loop(&element, entries, parts.chain(body).collect())
}

/// `for iterable {body}` binds the implicit `it`: `for 1..4 {x+=it}`
fn for_it(items: &[Node]) -> Option<Node> {
	let [keyword, iterable, body] = items else { return None };
	if !is_word(keyword, FOR_KEYWORD) || !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	for_in(&[keyword.clone(), symbol(IMPLICIT_VARIABLE), symbol(IN_KEYWORD), iterable.clone(), body.clone()])
}

fn counting_loop(variable: &Node, start: &Node, range: Op, end: &Node, mut body: Vec<Node>) -> Node {
	let test_op = if range == Op::To { Op::Le } else { Op::Lt };
	body.push(mark_step(key(variable.clone(), Op::Inc, Node::Empty)));
	block(vec![
		key(variable.clone(), Op::Assign, start.clone()),
		while_do(key(variable.clone(), test_op, end.clone()), block(body, Bracket::Curly)),
	], Bracket::None)
}

fn walking_loop(variable: &Node, iterable: Node, mut body: Vec<Node>) -> Node {
	let name = variable.name();
	let (items, index) = (symbol(&format!("{name}·items")), symbol(&format!("{name}·index")));
	let element = key(items.clone(), Op::Hash, key(index.clone(), Op::Add, number(1)));
	let mut statements = vec![key(variable.clone(), Op::Assign, element)];
	statements.append(&mut body);
	statements.push(mark_step(key(index.clone(), Op::Inc, Node::Empty)));
	let test = key(index.clone(), Op::Lt, key(Node::Empty, Op::Hash, items.clone()));
	block(vec![
		key(items, Op::Assign, iterable),
		key(index, Op::Assign, number(FIRST_INDEX)),
		while_do(test, block(statements, Bracket::Curly)),
	], Bracket::None)
}
