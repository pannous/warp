//! `for` loops lower to the `while` loop: a range counts its variable up, any other iterable is walked by an index.
//!
//! `for i in a..b body` → `i=a; while i<b {body; i++}` (`...` and `to` include b: `i<=b`), so `i` is b (b+1) afterwards;
//! b is evaluated once, before the loop, as in Python (`0..n`, `0..count(xs)`): `i=a; i·end=b; while i<i·end {…}`
//! `for x in xs body`   → `items=xs; index=0; while index<#items {x=items#(index+1); body; index++}`
//! `for(init;test;step){body}` → `init; while test {body; step}`

use super::nodes::{assign, block, call, for_in_variable, int, key, statement_list, symbol};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::{while_do, REVERSE_WORD};
use crate::wasm_emitter::{is_step, mark_step};
use crate::words::ORD_WORD;
use crate::words::{CHR_WORD, FOR_WORD, IMPLICIT_PARAMETER, IN_WORD};
use std::collections::HashSet;

pub mod headers;

const RANGE_CALL: &str = "range";
const FIRST_INDEX: i64 = 0;
/// `for c in x to 'e'`: the codepoint counter `c·code`, read with ord and made a letter with chr
const CODE_SUFFIX: &str = "·code";
/// `for c in w.chars`: walked, a unit count is the list of its items, `chars(w)`, `codepoint·list(w)`
const WALKED_UNITS: [(&str, &str); 2] = [
	(crate::analyzer::GRAPHEME_COUNT, crate::library_words::CHARS_WORD),
	(crate::analyzer::CODE_POINT_COUNT, crate::library_words::CODE_POINT_LIST),
];
/// The method an iterator object gives its next item with, ø at the end
const NEXT_METHOD: &str = "next";
/// The object a loop walks with next(), `x·iterator`
const ITERATOR_SUFFIX: &str = "·iterator";

/// The `while` lowering of a `for` loop, or the node itself when it is not one
/// The counter of a loop over a list's items, `x·index`: 0, then ++ while below the item count (wasm_emitter
/// bounded_counters proves it an i32, so its arithmetic needs no big-integer checks)
pub const INDEX_SUFFIX: &str = "·index";
/// The list a for loop walks, `x·items`, assigned once before the loop
pub const ITEMS_SUFFIX: &str = "·items";
/// The end of a counting loop, `i·end`, evaluated once before the loop as a walked list is (a literal stays inline)
const END_SUFFIX: &str = "·end";

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
	body.visit(&mut |part| {
		if let Node::List(items, _, _) = part {
			variables.extend(for_in_variable(items));
			for step in items.iter().filter(|item| is_step(item)) {
				if let Node::Key(counter, Op::Inc, _) = step.drop_meta() {
					let counter = counter.name();
					variables
						.extend(counter.strip_suffix(INDEX_SUFFIX).unwrap_or(&counter).split('·').map(str::to_string));
				}
			}
		}
	});
	variables
}

/// `for(init;test;step){body}`, parsed as `((for (init;test;step)) {body})`
fn classic_for(items: &[Node]) -> Option<Node> {
	let [header, body] = items else { return None };
	let Node::List(header, _, _) = header.drop_meta() else {
		return None;
	};
	let [keyword, parts] = header.as_slice() else {
		return None;
	};
	let Node::List(parts, Bracket::Round, Separator::Semicolon) = parts.drop_meta() else {
		return None;
	};
	let [init, test, step] = parts.as_slice() else {
		return None;
	};
	if !keyword.is_symbol(FOR_WORD) || !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	let mut statements = body.body_statements();
	statements.push(mark_step(step.clone()));
	Some(statement_list(vec![init.clone(), while_do(test.clone(), block(statements))], Bracket::None))
}

/// `for x in iterable {body}` and `for x in iterable: body`
fn for_in(items: &[Node]) -> Option<Node> {
	let [keyword, variable, in_word, rest @ ..] = items else {
		return None;
	};
	if !keyword.is_symbol(FOR_WORD) || !in_word.is_symbol(IN_WORD) {
		return None;
	}
	let names = loop_names(variable)?;
	let (iterable, body) = iterable_and_body(rest)?;
	let mut body = body.body_statements();
	// `for word in text: print it`: it is the loop variable too
	if matches!(names.as_slice(), [name] if name != IMPLICIT_PARAMETER && !crate::lambdas::is_inlined_loop_variable(&name.name()))
	{
		body = body.into_iter().map(|statement| it_as(statement, variable)).collect();
	}
	if names.len() > 1 {
		return Some(destructuring_loop(&names, iterable, body));
	}
	if let Some((high, low)) = falling_range(&iterable) {
		return Some(counter_loop(variable, &high, (Op::Ge, Op::Dec), &low, body));
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

/// What follows `for x in`: `iterable {body}`, `iterable: body`, or `iterable words…` whose words are the body
/// (`for i in 1 to 5 print i`, `for key in m print key`)
pub(crate) fn iterable_and_body(rest: &[Node]) -> Option<(Node, Node)> {
	match rest {
		[iterable, body] => Some((iterable.clone(), body.clone())),
		[iterable, words @ ..] if words.len() > 1 => {
			Some((iterable.clone(), Node::List(words.to_vec(), Bracket::None, Separator::Space)))
		}
		[colon] => match colon.drop_meta() {
			Node::Key(iterable, Op::Colon, body) => Some((iterable.as_ref().clone(), body.as_ref().clone())),
			_ => None,
		},
		_ => None,
	}
}

/// `it` replaced by the loop variable, except inside a block or a lambda of the body, whose own parameter it is (an
/// if's branches are no such blocks)
fn it_as(node: Node, variable: &Node) -> Node {
	match node {
		Node::Symbol(name) if name == IMPLICIT_PARAMETER => variable.clone(),
		// an if's branches are the loop's own statements: `if x > 2 { s += it }`
		Node::Key(left, op @ (Op::Then | Op::Else), right) => {
			key(it_as(*left, variable), op, branch_it_as(*right, variable))
		}
		Node::List(_, Bracket::Curly, _) | Node::Key(_, Op::Arrow, _) => node,
		other => other.map_children(|child| it_as(child, variable)),
	}
}

fn branch_it_as(branch: Node, variable: &Node) -> Node {
	match branch {
		Node::List(items, Bracket::Curly, separator) => {
			Node::List(items.into_iter().map(|item| it_as(item, variable)).collect(), Bracket::Curly, separator)
		}
		meta @ Node::Meta { .. } => meta.under_meta(|node| branch_it_as(node, variable)),
		other => it_as(other, variable),
	}
}

/// The range an iterable spells: `(a..b)` is `a..b`, Python's `range(n)` is `0..n` and `range(a, b)` is `a..b`
/// `'a' to 'e'` (Kotlin's `'a'..'e'`): the letters as a list, `..` without the last
fn letters(start: &Node, op: Op, end: &Node) -> Option<Node> {
	let (Node::Char(first), Node::Char(last)) = (start.drop_meta(), end.drop_meta()) else {
		return None;
	};
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
	let letter = assign(variable.clone(), call(CHR_WORD, vec![code.clone()]));
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
		Node::List(names, Bracket::Round | Bracket::None, _)
			if !names.is_empty() && names.iter().all(|name| matches!(name.drop_meta(), Node::Symbol(_))) =>
		{
			Some(names.clone())
		}
		_ => None,
	}
}

/// `for (r, c) in pairs {body}` → `for r·c in map_entries(pairs) {r = r·c#1; c = r·c#2; body}`:
/// each name is bound to its part of the element; the elements of a map are its `key:value` entries
fn destructuring_loop(names: &[Node], iterable: Node, body: Vec<Node>) -> Node {
	let element = symbol(&names.iter().map(Node::name).collect::<Vec<_>>().join("·"));
	let entries = call(crate::library_words::MAP_ENTRIES, vec![iterable]);
	let parts = names
		.iter()
		.enumerate()
		.map(|(position, name)| assign(name.clone(), key(element.clone(), Op::Hash, int(position as i64 + 1))));
	walking_loop(&element, entries, parts.chain(body).collect())
}

/// `for iterable {body}` binds the implicit `it`: `for 1..4 {x+=it}`
fn for_it(items: &[Node]) -> Option<Node> {
	let [keyword, iterable, body] = items else { return None };
	if !keyword.is_symbol(FOR_WORD) || !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	for_in(&[keyword.clone(), symbol(IMPLICIT_PARAMETER), symbol(IN_WORD), iterable.clone(), body.clone()])
}

/// `for i in 10 down to 1`, parsed as `reverse(1 to 10)`: the high end and the low end of a falling count of numbers
/// (letters keep the walked list)
fn falling_range(iterable: &Node) -> Option<(Node, Node)> {
	let Node::List(items, _, _) = iterable.drop_meta() else { return None };
	let [word, range] = items.as_slice() else { return None };
	match range.drop_meta() {
		Node::Key(low, Op::To, high) if word.is_symbol(REVERSE_WORD) && !is_letter(low) && !is_letter(high) => {
			Some((counted_start(high), (**low).clone()))
		}
		_ => None,
	}
}

fn counting_loop(variable: &Node, start: &Node, range: Op, end: &Node, body: Vec<Node>) -> Node {
	let test_op = if range == Op::To { Op::Le } else { Op::Lt };
	counter_loop(variable, start, (test_op, Op::Inc), end, body)
}

/// `variable = start; while variable <test> end do {body; variable <step>}`, the end held in a variable unless literal
fn counter_loop(variable: &Node, start: &Node, (test_op, step): (Op, Op), end: &Node, mut body: Vec<Node>) -> Node {
	body.push(mark_step(key(variable.clone(), step, Node::Empty)));
	let mut statements = vec![assign(variable.clone(), start.clone())];
	let is_literal = matches!(end.drop_meta(), Node::Number(_) | Node::Char(_));
	let end = if !is_literal {
		let end_variable = symbol(&format!("{}{END_SUFFIX}", variable.name()));
		statements.push(assign(end_variable.clone(), end.clone()));
		end_variable
	} else {
		end.clone()
	};
	statements.push(while_do(key(variable.clone(), test_op, end), block(body)));
	statement_list(statements, Bracket::None)
}

fn walked_units(iterable: Node) -> Node {
	let Node::Key(text, Op::Dot, unit) = iterable.drop_meta() else { return iterable };
	let counted = crate::analyzer::text_unit(&unit.drop_meta().name());
	match WALKED_UNITS.iter().find(|(unit, _)| Some(*unit) == counted) {
		Some((_, list)) => call(list, vec![(**text).clone()]),
		None => iterable,
	}
}

pub(crate) fn walking_loop(variable: &Node, iterable: Node, mut body: Vec<Node>) -> Node {
	// a walked literal of constants is named apart, so lifted_walked_literals may build it once before the outer loops
	let name = match is_constant_literal(&iterable) {
		true => format!("{}·{}", variable.name(), super::nodes::fresh_number()),
		false => variable.name(),
	};
	let (items, index) = (symbol(&format!("{name}{ITEMS_SUFFIX}")), symbol(&format!("{name}{INDEX_SUFFIX}")));
	let element = key(items.clone(), Op::Hash, key(index.clone(), Op::Add, int(1)));
	let mut statements = vec![assign(variable.clone(), element)];
	statements.append(&mut body);
	statements.push(mark_step(key(index.clone(), Op::Inc, Node::Empty)));
	let test = key(index.clone(), Op::Lt, key(Node::Empty, Op::Hash, items.clone()));
	statement_list(
		vec![assign(items, walked_units(iterable)), assign(index, int(FIRST_INDEX)), while_do(test, block(statements))],
		Bracket::None,
	)
}

/// Iterator objects : `for x in c {body}` of an object whose class has a method `next()`,
/// `c = Countdown(3)` or `for x in Countdown(3)`, walks what `next()` gives until it gives ø:
/// `x·iterator = c; x = x·iterator.next(); while x != ø { body; x = x·iterator.next() }` (the last a marked step,
/// which `continue` runs too). Before class_methods, which lowers the `next()` calls.
pub fn lower_iterators(node: Node) -> Node {
	let classes = iterator_classes(&node);
	if classes.is_empty() {
		return node;
	}
	let mut objects = HashSet::new();
	node.visit(&mut |part| {
		if let Node::Key(target, Op::Assign, value) = part {
			if constructed_iterator(value, &classes) {
				objects.extend(target.symbol_name().map(String::from));
			}
		}
	});
	iterator_loops(node, &classes, &objects)
}

/// The classes with a method `next()`
fn iterator_classes(node: &Node) -> HashSet<String> {
	let mut classes = HashSet::new();
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		let is_next = |member: &Node| matches!(member.drop_meta(), Node::Key(head, Op::Define, _)
			if matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() == 1 && items[0].is_symbol(NEXT_METHOD)));
		let members = match body.drop_meta() {
			Node::List(items, _, _) => items.clone(),
			single => vec![single.clone()],
		};
		if members.iter().any(is_next) {
			classes.extend(name.symbol_name().map(String::from));
		}
	});
	classes
}

/// `Countdown(3)` of an iterator class
fn constructed_iterator(value: &Node, classes: &HashSet<String>) -> bool {
	matches!(value.drop_meta(), Node::List(items, Bracket::Round | Bracket::None, _)
		if items.first().and_then(Node::symbol_name).is_some_and(|class| classes.contains(class)))
}

fn iterator_loops(node: Node, classes: &HashSet<String>, objects: &HashSet<String>) -> Node {
	let node = node.map_children(|child| iterator_loops(child, classes, objects));
	let Node::List(items, _, _) = node.drop_meta() else {
		return node;
	};
	let (variable, iterable, body) = match items.as_slice() {
		[keyword, variable, within, iterable, body] if keyword.is_symbol(FOR_WORD) && within.is_symbol(IN_WORD) => {
			(variable, iterable, body)
		}
		_ => return node,
	};
	let is_object =
		iterable.symbol_name().is_some_and(|name| objects.contains(name)) || constructed_iterator(iterable, classes);
	let Some(name) =
		variable.symbol_name().filter(|_| is_object && matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)))
	else {
		return node;
	};
	let iterator = symbol(&format!("{name}{ITERATOR_SUFFIX}"));
	let next = || assign(variable.clone(), key(iterator.clone(), Op::Dot, call(NEXT_METHOD, vec![])));
	let mut statements_of_body = body.body_statements();
	statements_of_body.push(mark_step(next()));
	let more = key(variable.clone(), Op::Ne, Node::Empty);
	statement_list(
		vec![assign(iterator.clone(), iterable.clone()), next(), while_do(more, block(statements_of_body))],
		Bracket::None,
	)
	.with_meta_of(&node)
}

/// `[16, 8, 0]`: a list of numbers, texts, codepoints and truth values, which no code changes
fn is_constant_literal(iterable: &Node) -> bool {
	let is_constant = |item: &Node| {
		matches!(item.drop_meta(), Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False)
	};
	matches!(iterable.drop_meta(), Node::List(items, Bracket::Square, _) if items.iter().all(is_constant))
}

/// `for i in 0..n { for shift in [16, 8, 0] {…} }` builds its walked literal once, before the outer loop, not each
/// round (card loop-literal-list): `shift·3·items = [16, 8, 0]` (walking_loop) lifts out of every loop around it.
/// Through statement lists only, never out of a block, so never out of a function or lambda
pub fn lifted_walked_literals(node: Node) -> Node {
	match node.map_children(lifted_walked_literals) {
		Node::List(statements, bracket, separator) if separator.separates_statements() => {
			let lifted =
				statements.into_iter().flat_map(|statement| match without_walked_literals(statement.clone()) {
					(_, literals) if literals.is_empty() || !is_while(&statement) => vec![statement],
					(rest, mut literals) => {
						literals.push(rest);
						literals
					}
				});
			Node::List(lifted.collect(), bracket, separator)
		}
		other => other,
	}
}

fn is_while(statement: &Node) -> bool {
	matches!(statement.drop_meta(), Node::Key(head, Op::Do, _) if matches!(head.drop_meta(), Node::Key(_, Op::While, _)))
}

/// The while loop without the walked literals its body assigns (directly or in a statement list of it), and those
fn without_walked_literals(statement: Node) -> (Node, Vec<Node>) {
	let Node::Key(head, Op::Do, body) = statement.drop_meta().clone() else {
		return (statement, vec![]);
	};
	let mut literals = vec![];
	let body = taken_walked_literals(*body, &mut literals);
	(Node::Key(head, Op::Do, Box::new(body)), literals)
}

fn taken_walked_literals(node: Node, literals: &mut Vec<Node>) -> Node {
	let Node::List(statements, bracket, separator) = node else {
		return node;
	};
	let mut kept = vec![];
	for statement in statements {
		match statement.drop_meta() {
			Node::Key(items, Op::Assign, value)
				if items.drop_meta().name().ends_with(ITEMS_SUFFIX) && is_constant_literal(value) =>
			{
				literals.push(statement)
			}
			Node::List(_, Bracket::None, inner) if inner.separates_statements() => {
				kept.push(taken_walked_literals(statement.drop_meta().clone(), literals))
			}
			_ => kept.push(statement),
		}
	}
	Node::List(kept, bracket, separator)
}
