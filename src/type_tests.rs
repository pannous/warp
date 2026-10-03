//! Type tests: `x is int`, `x is a number`, `[1 2] is list of int`, `[1 2] is ints` lower to `is_type(x, "spec")`, which the
//! emitter answers from the static type name of `x` (the same as `type(x)`). `type of x` is `type(x)`.
//! A type word on the right of `is` switches from equality to a type test; `x is y` with a variable stays equality.
//! `x == int` is no type test (user decision #30): a value never equals a type, so it is false and hints `x is int`.

use crate::analyzer::{extract_user_functions, plural_element_type, type_word_kind};
use crate::context::Context;
use crate::library_words::collect_assigned_names;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;

pub const IS_TYPE: &str = "is_type";
const ARTICLES: [&str; 2] = ["a", "an"];
const LIST_WORD: &str = "list";
const OF_WORD: &str = "of";
pub const TYPE_WORD: &str = "type";

/// Words that name the same type
fn canonical_spec_word(word: &str) -> &str {
	match word {
		"integer" | "long" | "i64" | "i32" => "int",
		"str" | "string" => "text",
		"char" => "codepoint",
		"double" | "f64" | "f32" | "fast" => "float",
		"exact" => "rational",
		other => other,
	}
}

/// Does a value of the static type name `actual` (`type(x)`) have the type `spec`: `number` covers every number, `real` the
/// exact numbers and π, `rational` the whole numbers too (int is a special case of rational), `list of number` every list of numbers
pub fn type_matches(actual: &str, spec: &str) -> bool {
	if let Some(conforms) = crate::traits::builtin_conforms(actual, spec) {
		return conforms;
	}
	if let Some(element) = spec.strip_prefix("list of ") {
		return actual.strip_prefix("list of ").is_some_and(|actual_element| type_matches(actual_element, element));
	}
	match spec {
		LIST_WORD => actual == LIST_WORD || actual.starts_with("list of "),
		"number" => ["int", "rational", "real", "float"].contains(&actual),
		"real" => ["int", "rational", "real"].contains(&actual),
		"rational" => ["int", "rational"].contains(&actual),
		"text" => ["text", "codepoint"].contains(&actual), // a one-character string is a codepoint
		other => actual == other,
	}
}

pub fn lower(node: Node) -> Node {
	let mut context = Context::new();
	extract_user_functions(&mut context, &node);
	let mut shadowed: HashSet<String> = context.user_functions.keys().cloned().collect();
	collect_assigned_names(&node, &mut shadowed);
	expand(node, &shadowed)
}

/// The spec of a type phrase (`int`, `a number`, `ints`, `list of int`, `list of list of int`), `None` when the words are no type
fn type_spec(words: &[&str], shadowed: &HashSet<String>) -> Option<String> {
	let words = match words {
		[article, rest @ ..] if ARTICLES.contains(article) && !rest.is_empty() => rest,
		all => all,
	};
	let (first, rest) = words.split_first()?;
	if shadowed.contains(*first) {
		return None;
	}
	match (*first, rest) {
		(LIST_WORD, []) => Some(LIST_WORD.to_string()),
		(LIST_WORD, [of, element @ ..]) if *of == OF_WORD => Some(format!("{LIST_WORD} of {}", type_spec(element, shadowed)?)),
		(word, []) => match plural_element_type(word) {
			Some(element) => Some(format!("{LIST_WORD} of {}", canonical_spec_word(element))),
			None if crate::traits::is_builtin_trait(word) => Some(word.to_string()),
			None => type_word_kind(word).map(|_| canonical_spec_word(word).to_string()),
		},
		_ => None,
	}
}

fn symbol_words(nodes: &[Node]) -> Option<Vec<&str>> {
	nodes.iter().map(|node| match node.drop_meta() {
		Node::Symbol(word) => Some(word.as_str()),
		_ => None,
	}).collect()
}

/// Meta key marking a type word compared with `==` (not `is`): only `is` tests types (user decision #30)
const EQUALITY_OPERAND: &str = "equality operand";

/// The right side of `x == word` as the parser marks it when the word names a type
pub fn equality_operand(word: Node) -> Node {
	match word.drop_meta() {
		Node::Symbol(name) if type_spec(&[name.as_str()], &HashSet::new()).is_some() => {
			Node::Meta { node: Box::new(word), data: Box::new(Node::key(EQUALITY_OPERAND, Node::True)) }
		}
		_ => word,
	}
}

fn is_equality_operand(node: &Node) -> bool {
	matches!(node, Node::Meta { data, .. } if matches!(data.as_ref(), Node::Key(key, _, _) if key.name() == EQUALITY_OPERAND))
}

/// `type(x) == int` compares two types; `x == int` compares a value with a type, never equal: educate toward `x is int`
fn compared_with_type(subject: Node, word: &Node, spec: String, shadowed: &HashSet<String>) -> Node {
	let compares_types = matches!(subject.drop_meta(), Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(head)) if head == TYPE_WORD));
	if compares_types {
		return is_type_call(expand(subject, shadowed), spec);
	}
	let (written, preferred) = (crate::normalize::operand_text(&subject), word.drop_meta().name());
	crate::normalize::hint(&format!("{written} == {preferred}"), &format!("{written} is {preferred}"), "only `is` tests a type; a value never equals a type");
	Node::False
}

fn is_type_call(subject: Node, spec: String) -> Node {
	Node::List(vec![Node::Symbol(IS_TYPE.to_string()), subject, Node::Text(spec)], Bracket::Round, Separator::None)
}

fn expand(node: Node, shadowed: &HashSet<String>) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			if let Some(call) = type_of_word(&items, shadowed).or_else(|| spaced_type_test(&items, shadowed)) {
				return expand(call, shadowed);
			}
			Node::List(items.into_iter().map(|item| expand(item, shadowed)).collect(), bracket, separator)
		}
		Node::Key(subject, Op::Eq, right) => match symbol_words(std::slice::from_ref(&*right)).and_then(|words| type_spec(&words, shadowed)) {
			Some(spec) if is_equality_operand(&right) => compared_with_type(*subject, &right, spec, shadowed),
			Some(spec) => is_type_call(expand(*subject, shadowed), spec),
			None => Node::Key(Box::new(expand(*subject, shadowed)), Op::Eq, Box::new(expand(*right, shadowed))),
		},
		Node::Key(left, op, right) => Node::Key(Box::new(expand(*left, shadowed)), op, Box::new(expand(*right, shadowed))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(expand(*node, shadowed)), data },
		other => other,
	}
}

/// `x is a number` is the items `x is a` and `number`: the words after the comparison continue the type phrase
fn spaced_type_test(items: &[Node], shadowed: &HashSet<String>) -> Option<Node> {
	let [first, tail @ ..] = items else { return None };
	let Node::Key(subject, Op::Eq, right) = first.drop_meta() else { return None };
	let Node::Symbol(word) = right.drop_meta() else { return None };
	let mut words = vec![word.as_str()];
	words.extend(symbol_words(tail)?);
	Some(is_type_call(subject.as_ref().clone(), type_spec(&words, shadowed)?))
}

/// `type of x`: the parser reads `type of` as a declaration head, its argument is the next item
fn type_of_word(items: &[Node], shadowed: &HashSet<String>) -> Option<Node> {
	let [head, argument @ ..] = items else { return None };
	if argument.is_empty() || shadowed.contains(TYPE_WORD) {
		return None;
	}
	let Node::Type { name, body } = head.drop_meta() else { return None };
	if !matches!(name.drop_meta(), Node::Symbol(word) if word == OF_WORD) || !matches!(body.drop_meta(), Node::Empty) {
		return None;
	}
	let argument = match argument {
		[single] => single.clone(),
		many => Node::List(many.to_vec(), Bracket::None, Separator::Space),
	};
	Some(Node::List(vec![Node::Symbol(TYPE_WORD.to_string()), argument], Bracket::Round, Separator::None))
}
