//! One rule (card cleanup-of-is-dot): `a of b` is `b.a`. `name of p` is `p.name`, `first of xs` is `xs.first`,
//! `first 3 of xs` is `xs.first(3)`, `a of b of c` is `c.b.a`; the passes after this one read only the dot form.
//! `of` binds tighter than any operator: `name of p + 1` is `p.name + 1`, `listeners of alarm -= h` is
//! `alarm.listeners -= h`. Not lookups: a type phrase `list of int` (a type word before `of`) and a spaced definition
//! `square of a number = …`. The object is one item with its qualifier: `name of people with age > 20` reads the field of
//! the filtered list. `type of x` is the call `type(x)`, so both spellings answer alike (card type-of-consistency).
//! `in` and `from` select the same way after a selector word the program does not define (card of-in-chain):
//! `length of characters in text` is `text.characters.length`; otherwise `x in xs` stays the membership test, and the
//! walk `print words in t` prints each word.

use super::nodes::{call, children_rewritten, spaced_statement};
use crate::analyzer::is_counting_property;
use crate::lowering::comprehensions::{WHERE_KEYWORD, WITHOUT_WORD, WITH_WORD};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{FOR_WORD, FROM_WORD, IN_WORD, OF_WORD, PRINT_WORD};
use std::collections::HashSet;

/// The words naming a selection of a value's parts, which `in` and `from` select like `of`: `words in t`
const SELECTOR_WORDS: [&str; 7] = ["chars", "characters", "letters", "words", "lines", "keys", "values"];
const SELECTING_PREPOSITIONS: [&str; 2] = [IN_WORD, FROM_WORD];
/// `print all words in t`: the walk's optional word before the selection
const ALL_WORD: &str = "all";
/// The loop variable of the walk `print words in t`, numbered fresh
const WALKED_PART: &str = "walked_part_";

pub fn lower(node: Node) -> Node {
	let mut has_type_of = false;
	node.visit(&mut |part| has_type_of |= type_of_body(part).is_some());
	let has_selection = node.mentions_any(&SELECTOR_WORDS) && node.mentions_any(&SELECTING_PREPOSITIONS);
	if !has_type_of && !has_selection && !node.mentions_any(&[OF_WORD]) {
		return node;
	}
	let mut defined = crate::library_words::defined_names(&node);
	defined.extend(loop_variables(&node));
	let selectors: Vec<&str> = SELECTOR_WORDS.into_iter().filter(|word| !defined.contains(*word)).collect();
	rewrite(node, &selectors)
}

fn rewrite(node: Node, selectors: &[&str]) -> Node {
	if let Some(walk) = print_walk(&node, selectors) {
		return walk;
	}
	match children_rewritten(as_one_phrase(node), |child| rewrite(child, selectors)) {
		// a phrase, a call's arguments `str(name of people)`, a block of one statement `{remove b from listeners of t}`
		Node::List(
			items,
			bracket @ (Bracket::None | Bracket::Round | Bracket::Curly),
			separator @ (Separator::Space | Separator::None),
		) if spaced_statement(&items).is_none() => {
			let items = looked_up(type_of_called(items), selectors);
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

/// An assigned value arrives grouped, `x = sum of xs` as `sum (of xs)`, `x = name of q with a` as
/// `(((name of) q) with) a`: a phrase with an `of` is read as its words
fn as_one_phrase(node: Node) -> Node {
	if !matches!(node.drop_meta(), Node::List(_, Bracket::None, Separator::Space)) {
		return node;
	}
	let mut words = vec![];
	crate::lambdas::flatten_phrase(&node, &mut words);
	match words.iter().any(|word| word.is_symbol(OF_WORD)) {
		true => Node::List(words, Bracket::None, Separator::Space),
		false => node,
	}
}

/// `print words in t`, `print all words in t`: the walk printing each selected part, `for part in t.words {print part}`
/// (as counting print_walk does for a name no variable has)
fn print_walk(node: &Node, selectors: &[&str]) -> Option<Node> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let [print, phrase] = items.as_slice() else { return None };
	let Node::List(words, _, _) = phrase.drop_meta() else { return None };
	let selection = match words.as_slice() {
		[all, rest @ ..] if all.is_symbol(ALL_WORD) => rest,
		words => words,
	};
	// `print all words in t` arrives as `all (words in t)`
	let selection = match selection {
		[grouped] => match grouped.drop_meta() {
			Node::List(words, Bracket::None, _) => words.as_slice(),
			_ => selection,
		},
		_ => selection,
	};
	let [selector, _, value] = selection else { return None };
	if !print.is_symbol(PRINT_WORD) || !is_selector_preposition(selection, 1, selectors) {
		return None;
	}
	let selected = member_of(rewrite(value.clone(), selectors), selector.clone());
	let part = Node::Symbol(format!("{WALKED_PART}{}", crate::lowering::nodes::fresh_number()));
	let body = Node::List(vec![print.clone(), part.clone()], Bracket::Curly, Separator::Space);
	let words = [Node::Symbol(FOR_WORD.to_string()), part, Node::Symbol(IN_WORD.to_string()), selected, body];
	Some(Node::List(words.to_vec(), Bracket::None, Separator::Space))
}

/// The names a loop binds, `for words in xs`: a selector word bound so is the program's name
fn loop_variables(node: &Node) -> HashSet<String> {
	let mut bound = HashSet::new();
	node.visit(&mut |part| {
		if let Node::List(items, _, _) = part {
			for pair in items.windows(2).filter(|pair| pair[0].is_symbol(FOR_WORD)) {
				bound.extend(pair[1].symbol_name().map(str::to_string));
			}
		}
	});
	bound
}

/// `of` after anything but a type head, `in` and `from` after a selector word: `words in t`
fn is_selection(items: &[Node], at: usize, selectors: &[&str]) -> bool {
	let before = &items[at - 1];
	match items[at].symbol_name() {
		Some(OF_WORD) => !is_type_head(before),
		_ if is_selector_preposition(items, at, selectors) => true,
		_ => false,
	}
}

fn is_selector_preposition(items: &[Node], at: usize, selectors: &[&str]) -> bool {
	items[at].symbol_name().is_some_and(|word| SELECTING_PREPOSITIONS.contains(&word))
		&& items[at - 1].symbol_name().is_some_and(|name| selectors.contains(&name))
}

/// The items with each `a of b` as `b.a`, the last `of` first: `a of b of c` is `a of c.b`, then `c.b.a`; `in` and
/// `from` after a selector word alike
fn looked_up(mut items: Vec<Node>, selectors: &[&str]) -> Vec<Node> {
	while let Some(at) = (1..items.len().saturating_sub(1)).rev().find(|at| is_selection(&items, *at, selectors)) {
		let object = noun_phrase(items.drain(at + 1..phrase_end(&items, at + 1)).collect());
		items.remove(at);
		let word = items.remove(at - 1);
		let before = at.checked_sub(2).and_then(|before| items[before].symbol_name());
		let unit = before
			.and_then(crate::analyzer::text_unit)
			.filter(|_| word.symbol_name().is_some_and(is_counting_property));
		let (start, member) = match (before, word.drop_meta(), unit) {
			// `byte count of t`: the count in that unit, `t.bytes`
			(_, _, Some(unit)) => {
				items.remove(at - 2);
				(at - 2, Node::Symbol(unit.to_string()))
			}
			// `first 3 of xs`: the word before a value takes it, `xs.first(3)`
			(Some(head), argument, None) if !matches!(argument, Node::Symbol(_)) => {
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
		_ => Node::Key(Box::new(object), Op::Dot, Box::new(member)),
	}
}

/// `list of int`, `x: list of int`: the word before `of` names a collection type, the phrase is a type
fn is_type_head(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(word) => crate::type_tests::COLLECTION_WORDS.contains(&crate::warp_parser::list_type_head(word)),
		Node::Key(_, Op::Colon, _) => true,
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
