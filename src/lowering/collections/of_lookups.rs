//! One rule (card cleanup-of-is-dot): `a of b` is `b.a`. `name of p` is `p.name`, `first of xs` is `xs.first`,
//! `first 3 of xs` is `xs.first(3)`, `a of b of c` is `c.b.a`; the passes after this one read only the dot form.
//! `of` binds tighter than any operator: `name of p + 1` is `p.name + 1`, `listeners of alarm -= h` is
//! `alarm.listeners -= h`. Not lookups: a type phrase `list of int` (a type word before `of`) and a spaced definition
//! `square of a number = …` (a type word names its parameter). A lookup is an assignment target too:
//! `width of p = 12` is `p.width = 12` (decision setter-calls). The object is one item with its qualifier: `name of people with age > 20` reads the field of
//! the filtered list. `type of x` is the call `type(x)`, so both spellings answer alike (card type-of-consistency).
//! `in` and `from` select the same way after a selector word the program does not define (card of-in-chain):
//! `length of characters in text` is `text.characters.length`; otherwise `x in xs` stays the membership test.
//! Dropped (decision sugar-triage): the walk `print words in t` and the unit count `byte count of t`.

use super::nodes::{call, children_rewritten, key, spaced_statement};
use crate::lowering::comprehensions::{WHERE_KEYWORD, WITHOUT_WORD, WITH_WORD};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{FROM_WORD, IN_WORD, OF_WORD};

/// The words naming a selection of a value's parts, which `in` and `from` select like `of`: `words in t`
pub const SELECTOR_WORDS: [&str; 11] = [
	"chars",
	"characters",
	"glyphs",
	"graphemes",
	"codepoints",
	"bytes",
	"letters",
	"words",
	"lines",
	"keys",
	"values",
];
const SELECTING_PREPOSITIONS: [&str; 2] = [IN_WORD, FROM_WORD];

pub fn lower(node: Node) -> Node {
	let mut has_type_of = false;
	node.visit(&mut |part| has_type_of |= type_of_body(part).is_some());
	let has_selection = node.mentions_any(&SELECTOR_WORDS) && node.mentions_any(&SELECTING_PREPOSITIONS);
	if !has_type_of && !has_selection && !node.mentions_any(&[OF_WORD]) {
		return node;
	}
	let defined = crate::program_names::Bindings::of(&node).bound();
	let selectors: Vec<&str> = SELECTOR_WORDS.into_iter().filter(|word| !defined.contains(*word)).collect();
	rewrite(node, &selectors)
}

fn rewrite(node: Node, selectors: &[&str]) -> Node {
	match children_rewritten(as_one_phrase(node), |child| rewrite(child, selectors)) {
		// a phrase, a call's arguments `str(name of people)`, a block of one statement `{remove b from listeners of t}`
		Node::List(
			items,
			bracket @ (Bracket::None | Bracket::Round | Bracket::Curly),
			separator @ (Separator::Space | Separator::None),
		) if !is_spaced_definition(&items) => {
			let items = folded_casts(looked_up(type_of_called(items), selectors));
			match (items.len(), &bracket) {
				(1, Bracket::None) => items.into_iter().next().expect("one item"),
				_ => Node::List(items, bracket, separator),
			}
		}
		// `type of {a:1}`: the map is the declaration head's body
		other if type_of_body(&other).is_some() => type_of_called(vec![other]).pop().expect("one item"),
		other => other,
	}
}

/// `square of a number = …`, `f x = …`: a definition head; `width of p = 12` without a type word assigns the lookup
fn is_spaced_definition(items: &[Node]) -> bool {
	spaced_statement(items).is_some_and(|(_, words, body)| {
		!words.contains(&OF_WORD) || crate::lowering::type_name_matching::spaced_parameters(&words, body).is_some()
	})
}

/// An assigned value arrives grouped, `x = sum of xs` as `sum (of xs)`, `x = name of q with a` as
/// `(((name of) q) with) a`: a phrase with an `of` is read as its words
fn as_one_phrase(node: Node) -> Node {
	if !matches!(node.drop_meta(), Node::List(_, Bracket::None, Separator::Space)) {
		return node;
	}
	let words = crate::lowering::nodes::phrase_words_of(&node);
	match words.iter().any(|word| word.is_symbol(OF_WORD)) {
		true => Node::List(words, Bracket::None, Separator::Space),
		false => node,
	}
}

/// `of` after anything but a type head, `in` and `from` after a selector word: `words in t`
fn is_selection(items: &[Node], at: usize, selectors: &[&str]) -> bool {
	let before = &items[at - 1];
	match items[at].symbol_name() {
		Some(OF_WORD) => !is_type_head(before),
		Some(word) if SELECTING_PREPOSITIONS.contains(&word) => {
			before.symbol_name().is_some_and(|name| selectors.contains(&name))
		}
		_ => false,
	}
}

/// The items with each `a of b` as `b.a`, the last `of` first: `a of b of c` is `a of c.b`, then `c.b.a`; `in` and
/// `from` after a selector word alike
fn looked_up(mut items: Vec<Node>, selectors: &[&str]) -> Vec<Node> {
	while let Some(at) = (1..items.len().saturating_sub(1)).rev().find(|at| is_selection(&items, *at, selectors)) {
		let object = noun_phrase(items.drain(at + 1..phrase_end(&items, at + 1)).collect());
		items.remove(at);
		let word = items.remove(at - 1);
		let before = at.checked_sub(2).and_then(|before| items[before].symbol_name());
		let (start, member) = match (before, word.drop_meta()) {
			// `first 3 of xs`: the word before a value takes it, `xs.first(3)`
			(Some(head), argument) if !matches!(argument, Node::Symbol(_)) => {
				let member = call(head, vec![word]);
				items.remove(at - 2);
				(at - 2, member)
			}
			_ => (at - 1, word),
		};
		items.insert(start, member_of(object, member));
	}
	items
}

/// `[] as list of int`, parsed as the items `[] as list`, `of`, `int` (`xs = [] as list`, `of`, `int` in an
/// assignment): the cast to the one type `list of int`, `list of list of int` alike
fn folded_casts(mut items: Vec<Node>) -> Vec<Node> {
	let mut at = 0;
	while at + 2 < items.len() {
		let element = items[at + 2].symbol_name().filter(|_| items[at + 1].is_symbol(OF_WORD));
		match element.and_then(|element| with_cast_of(&items[at], element)) {
			Some(cast) => {
				items[at] = cast;
				items.drain(at + 1..at + 3);
			}
			None => at += 1,
		}
	}
	items
}

/// The item ending in a cast to a collection type, `[] as list` or `xs = [] as list`, with that type made `list of element`
fn with_cast_of(item: &Node, element: &str) -> Option<Node> {
	match item.drop_meta() {
		Node::Key(value, Op::As, type_word) => {
			let head = type_word.symbol_name()?;
			let first = head.split(' ').next()?;
			is_type_head(&Node::Symbol(first.to_string())).then(|| {
				let head = crate::warp_parser::list_type_head(head);
				key(value.as_ref().clone(), Op::As, Node::Symbol(format!("{head} {OF_WORD} {element}")))
			})
		}
		Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
			Some(Node::Key(target.clone(), *op, Box::new(with_cast_of(value, element)?)))
		}
		_ => None,
	}
}

/// `type of x` as `type(x)`, `print type of x` as `print type(x)`: the parser reads `type of` as a declaration head
/// whose argument is its body (`type of {a:1}`) or the rest of the phrase
fn type_of_called(mut items: Vec<Node>) -> Vec<Node> {
	let Some(at) = items.iter().position(|item| type_of_body(item).is_some()) else { return items };
	let body = type_of_body(&items[at]).expect("a type of head").clone();
	let rest: Vec<Node> = items.drain(at + 1..).collect();
	let argument = match (body.is_empty_node(), rest.is_empty()) {
		(false, true) => body,
		(true, false) => noun_phrase(rest),
		_ => return items.into_iter().chain(rest).collect(),
	};
	items[at] = call(crate::type_tests::TYPE_WORD, vec![argument]);
	items
}

fn type_of_body(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		Node::Type { name, body } if name.is_symbol(OF_WORD) => Some(body.as_ref()),
		_ => None,
	}
}

/// `object.member`, through the left operands of an operator: `p + 1` gives `p.name + 1`
fn member_of(object: Node, member: Node) -> Node {
	match object.drop_meta() {
		Node::Key(left, op, right) if *op != Op::Dot && !op.is_prefix() && !matches!(left.drop_meta(), Node::Empty) => {
			Node::Key(Box::new(member_of(left.as_ref().clone(), member)), *op, right.clone())
		}
		_ => key(object, Op::Dot, member),
	}
}

/// `list of int`, `x: list of int`: the word before `of` names a collection type, the phrase is a type
fn is_type_head(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(word) => crate::type_tests::COLLECTION_WORDS.contains(&crate::warp_parser::list_type_head(word)),
		Node::Key(_, Op::Colon, _) => true,
		// `[] as list of int`: the cast's type
		Node::Key(_, Op::As, type_word) => is_type_head(type_word),
		_ => false,
	}
}

/// Where the object starting `at` ends: after its item, or with a qualifier (`people where …`) at the end of the phrase
fn phrase_end(items: &[Node], at: usize) -> usize {
	let qualified = items
		.get(at + 1)
		.is_some_and(|word| [WHERE_KEYWORD, WITH_WORD, WITHOUT_WORD].iter().any(|qualifier| word.is_symbol(qualifier)));
	if qualified {
		items.len()
	} else {
		at + 1
	}
}

fn noun_phrase(mut words: Vec<Node>) -> Node {
	match words.len() {
		1 => words.pop().expect("one word"),
		_ => Node::List(words, Bracket::None, Separator::Space),
	}
}
