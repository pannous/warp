//! `for` loops lower to the `while` loop: a range counts its variable up, any other iterable is walked by an index.
//!
//! `for i in a..b body` → `i=a; while i<b {body; i++}` (`...` and `to` include b: `i<=b`), so `i` is b (b+1) afterwards
//! `for x in xs body`   → `items=xs; index=0; while index<#items {x=items#(index+1); body; index++}`
//! `for(init;test;step){body}` → `init; while test {body; step}`

use super::nodes::{call, int, key, statement_list, symbol};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasm_emitter::{is_step, mark_step};
use crate::warp_parser::while_do;

const FOR_KEYWORD: &str = "for";
const IN_KEYWORD: &str = "in";
const IMPLICIT_VARIABLE: &str = "it";
const RANGE_CALL: &str = "range";
const FIRST_INDEX: i64 = 0;
/// `for c in x to 'e'`: the codepoint counter `c·code`, read with ord and made a letter with chr
const CODE_SUFFIX: &str = "·code";
const ORD_WORD: &str = "ord";
const CHR_WORD: &str = "chr";
/// `for c in w.chars`, `w.chars.map(…)`: walked, the unit word `w.chars` (elsewhere the count) is the list `chars(w)`
const CHARS_CALL: &str = "chars";
const CODEPOINT_UNIT: &str = "codepoints";

/// The `while` lowering of a `for` loop, or the node itself when it is not one
/// The counter of a loop over a list's items, `x·index`: 0, then ++ while below the item count (wasm_emitter
/// bounded_counters proves it an i32, so its arithmetic needs no big-integer checks)
pub const INDEX_SUFFIX: &str = "·index";
/// The list a for loop walks, `x·items`, assigned once before the loop
pub const ITEMS_SUFFIX: &str = "·items";

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

/// The loop variables of every `for … in` in `body`, written or already lowered (its marked step `x++`, `x·index++`, of a
/// destructuring `k·v·index++`): the loop's own names, never an outer variable
pub(crate) fn loop_variables(body: &Node) -> Vec<String> {
	let mut variables = vec![];
	body.visit(&mut |part| if let Node::List(items, _, _) = part {
		if let [keyword, Node::Symbol(variable), within, ..] = items.iter().map(Node::drop_meta).collect::<Vec<_>>().as_slice() {
			if keyword.is_symbol(FOR_KEYWORD) && within.is_symbol(IN_KEYWORD) {
				variables.push(variable.clone());
			}
		}
		for step in items.iter().filter(|item| is_step(item)) {
			if let Node::Key(counter, Op::Inc, _) = step.drop_meta() {
				let counter = counter.name();
				variables.extend(counter.strip_suffix(INDEX_SUFFIX).unwrap_or(&counter).split('·').map(str::to_string));
			}
		}
	});
	variables
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
	if !keyword.is_symbol(FOR_KEYWORD) || !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	let mut statements = block_items(body);
	statements.push(mark_step(step.clone()));
	Some(statement_list(vec![init.clone(), while_do(test.clone(), statement_list(statements, Bracket::Curly))], Bracket::None))
}

/// `for x in iterable {body}` and `for x in iterable: body`
fn for_in(items: &[Node]) -> Option<Node> {
	let [keyword, variable, in_word, rest @ ..] = items else { return None };
	if !keyword.is_symbol(FOR_KEYWORD) || !in_word.is_symbol(IN_KEYWORD) {
		return None;
	}
	let names = loop_names(variable)?;
	let (iterable, body) = match rest {
		[iterable, body] => (iterable.clone(), body.clone()),
		// `for i in 1 to 5\n  puti i`: the words after the iterable are the body
		[iterable, words @ ..] if words.len() > 1 => (iterable.clone(), Node::List(words.to_vec(), Bracket::None, Separator::Space)),
		[colon] => match colon.drop_meta() {
			Node::Key(iterable, Op::Colon, body) => (iterable.as_ref().clone(), body.as_ref().clone()),
			_ => return None,
		},
		_ => return None,
	};
	let mut body = block_items(&body);
	// `for word in text: print it`: it is the loop variable too
	if matches!(names.as_slice(), [name] if name != IMPLICIT_VARIABLE && !crate::lambdas::is_inlined_loop_variable(&name.name())) {
		body = body.into_iter().map(|statement| it_as(statement, variable)).collect();
	}
	if names.len() > 1 {
		return Some(destructuring_loop(&names, iterable, body));
	}
	Some(match counting_range(&iterable).drop_meta() {
		Node::Key(start, op @ (Op::Range | Op::To), end) => match letters(start, *op, end) {
			Some(letters) => walking_loop(variable, letters, body),
			None if is_letter(start) || is_letter(end) => letter_loop(variable, start, *op, end, body),
			None => counting_loop(variable, &counted_start(start), *op, end, body),
		},
		_ => walking_loop(variable, iterable, body),
	})
}

/// `it` replaced by the loop variable, except inside a block or a lambda of the body, whose own parameter it is (an
/// if's branches are no such blocks)
fn it_as(node: Node, variable: &Node) -> Node {
	match node {
		Node::Symbol(name) if name == IMPLICIT_VARIABLE => variable.clone(),
		// an if's branches are the loop's own statements: `if x > 2 { s += it }`
		Node::Key(left, op @ (Op::Then | Op::Else), right) => key(it_as(*left, variable), op, branch_it_as(*right, variable)),
		Node::List(_, Bracket::Curly, _) | Node::Key(_, Op::Arrow | Op::FatArrow, _) => node,
		other => other.map_children(|child| it_as(child, variable)),
	}
}

fn branch_it_as(branch: Node, variable: &Node) -> Node {
	match branch {
		Node::List(items, Bracket::Curly, separator) => Node::List(items.into_iter().map(|item| it_as(item, variable)).collect(), Bracket::Curly, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(branch_it_as(*node, variable)), data },
		other => it_as(other, variable),
	}
}

/// The range an iterable spells: `(a..b)` is `a..b`, Python's `range(n)` is `0..n` and `range(a, b)` is `a..b`
/// `'a' to 'e'` (Kotlin's `'a'..'e'`): the letters as a list, `..` without the last
fn letters(start: &Node, op: Op, end: &Node) -> Option<Node> {
	let (Node::Char(first), Node::Char(last)) = (start.drop_meta(), end.drop_meta()) else { return None };
	let last = if op == Op::To { *last as u32 } else { (*last as u32).checked_sub(1)? };
	let letters = (*first as u32..=last).filter_map(char::from_u32).map(Node::Char).collect();
	Some(Node::List(letters, Bracket::Square, Separator::Space))
}

fn is_letter(bound: &Node) -> bool {
	matches!(bound.drop_meta(), Node::Char(_))
}

/// `c to 'e'` of a letter held in a variable: counting the codepoints, the loop variable the letter of each
fn letter_loop(variable: &Node, start: &Node, op: Op, end: &Node, body: Vec<Node>) -> Node {
	let code = symbol(&format!("{}{CODE_SUFFIX}", variable.name()));
	let codepoint = |letter: &Node| call(ORD_WORD, vec![letter.clone()]);
	let letter = key(variable.clone(), Op::Assign, call(CHR_WORD, vec![code.clone()]));
	counting_loop(&code, &codepoint(start), op, &codepoint(end), [vec![letter], body].concat())
}

/// A variable start counts as a number: `x to 3` of an undefined x is the error "undefined variable: x" (and a
/// character in it the error of a character in arithmetic), not a counter of the wrong type
fn counted_start(start: &Node) -> Node {
	match start.drop_meta() {
		Node::Symbol(_) => key(start.clone(), Op::Add, int(0)),
		_ => start.clone(),
	}
}

fn counting_range(iterable: &Node) -> Node {
	match iterable.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => counting_range(&items[0]),
		Node::List(items, _, _) if items[0].is_symbol(RANGE_CALL) => match &items[1..] {
			[end] => key(int(FIRST_INDEX), Op::Range, end.clone()),
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
	let parts = names.iter().enumerate().map(|(position, name)| key(name.clone(), Op::Assign, key(element.clone(), Op::Hash, int(position as i64 + 1))));
	walking_loop(&element, entries, parts.chain(body).collect())
}

/// `for iterable {body}` binds the implicit `it`: `for 1..4 {x+=it}`
fn for_it(items: &[Node]) -> Option<Node> {
	let [keyword, iterable, body] = items else { return None };
	if !keyword.is_symbol(FOR_KEYWORD) || !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	for_in(&[keyword.clone(), symbol(IMPLICIT_VARIABLE), symbol(IN_KEYWORD), iterable.clone(), body.clone()])
}

fn counting_loop(variable: &Node, start: &Node, range: Op, end: &Node, mut body: Vec<Node>) -> Node {
	let test_op = if range == Op::To { Op::Le } else { Op::Lt };
	body.push(mark_step(key(variable.clone(), Op::Inc, Node::Empty)));
	statement_list(vec![
		key(variable.clone(), Op::Assign, start.clone()),
		while_do(key(variable.clone(), test_op, end.clone()), statement_list(body, Bracket::Curly)),
	], Bracket::None)
}

fn walked_units(iterable: Node) -> Node {
	match iterable.drop_meta() {
		Node::Key(text, Op::Dot, unit) if crate::analyzer::text_unit(&unit.drop_meta().name()) == Some(CODEPOINT_UNIT) => call(CHARS_CALL, vec![(**text).clone()]),
		_ => iterable,
	}
}

fn walking_loop(variable: &Node, iterable: Node, mut body: Vec<Node>) -> Node {
	let name = variable.name();
	let (items, index) = (symbol(&format!("{name}{ITEMS_SUFFIX}")), symbol(&format!("{name}{INDEX_SUFFIX}")));
	let element = key(items.clone(), Op::Hash, key(index.clone(), Op::Add, int(1)));
	let mut statements = vec![key(variable.clone(), Op::Assign, element)];
	statements.append(&mut body);
	statements.push(mark_step(key(index.clone(), Op::Inc, Node::Empty)));
	let test = key(index.clone(), Op::Lt, key(Node::Empty, Op::Hash, items.clone()));
	statement_list(vec![
		key(items, Op::Assign, walked_units(iterable)),
		key(index, Op::Assign, int(FIRST_INDEX)),
		while_do(test, statement_list(statements, Bracket::Curly)),
	], Bracket::None)
}
