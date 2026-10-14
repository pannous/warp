//! One rule (card cleanup-of-is-dot): `a of b` is `b.a`. `name of p` is `p.name`, `first of xs` is `xs.first`,
//! `first 3 of xs` is `xs.first(3)`, `a of b of c` is `c.b.a`; the passes after this one read only the dot form.
//! `of` binds tighter than any operator: `name of p + 1` is `p.name + 1`, `listeners of alarm -= h` is
//! `alarm.listeners -= h`. Not lookups: a type phrase `list of int` (a type word before `of`) and a spaced definition
//! `square of a number = …`. The object is one item with its qualifier: `name of people with age > 20` reads the field of
//! the filtered list. `type of x` is the call `type(x)`, so both spellings answer alike (card type-of-consistency).

use super::nodes::{call, children_rewritten, spaced_statement};
use crate::analyzer::is_counting_property;
use crate::lowering::comprehensions::{WHERE_KEYWORD, WITHOUT_WORD, WITH_WORD};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::OF_WORD;

pub fn lower(node: Node) -> Node {
	let mut has_type_of = false;
	node.visit(&mut |part| has_type_of |= type_of_body(part).is_some());
	if !has_type_of && !node.mentions_any(&[OF_WORD]) {
		return node;
	}
	rewrite(node)
}

fn rewrite(node: Node) -> Node {
	match children_rewritten(as_one_phrase(node), rewrite) {
		// a phrase, a call's arguments `str(name of people)`, a block of one statement `{remove b from listeners of t}`
		Node::List(
			items,
			bracket @ (Bracket::None | Bracket::Round | Bracket::Curly),
			separator @ (Separator::Space | Separator::None),
		) if spaced_statement(&items).is_none() => {
			let items = looked_up(type_of_called(items));
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

/// The items with each `a of b` as `b.a`, the last `of` first: `a of b of c` is `a of c.b`, then `c.b.a`
fn looked_up(mut items: Vec<Node>) -> Vec<Node> {
	while let Some(at) = (1..items.len().saturating_sub(1))
		.rev()
		.find(|at| items[*at].is_symbol(OF_WORD) && !is_type_head(&items[at - 1]))
	{
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
