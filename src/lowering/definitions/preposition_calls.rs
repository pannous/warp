//! Prepositions name arguments (card prepositions-call, user 2026-10-14): a parameter written after a preposition,
//! `play(tone, for duration)`, is passed by that word as Swift passes its argument labels: `play 440Hz for 0.5s`,
//! `play(440Hz, for: 0.5s)` and `play tone:440Hz for:0.5s` are all `play(440Hz, duration: 0.5s)`, which
//! named_arguments.rs orders. A call without brackets also takes named arguments, `f b=1 a=5` is `f(b=1, a=5)`.
//! The word is a preposition only in a call of a function declaring it: `for x in xs` stays a loop, `1 to 5` a range
//! (P52), unless a call of a function with a `to` parameter holds it, `move x from 1 to 5`.

use super::nodes::{key, parameter_word};
use crate::errors::diagnostic::Diagnostic;
use crate::node::{parens, Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_name_matching::PREPOSITIONS;
use std::collections::{HashMap, HashSet};

/// A function's parameters by name, and which name each preposition passes, over all its overloads
#[derive(Default)]
struct Labels {
	parameters: Vec<String>,
	by_preposition: HashMap<String, String>,
}

struct Functions {
	labels: HashMap<String, Labels>,
	/// the program's variables: `by` assigned is no preposition
	variables: HashSet<String>,
}

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&PREPOSITIONS) && !has_spaced_named_call(&node) {
		return node;
	}
	let labels = functions(&node);
	if labels.is_empty() {
		return node;
	}
	let variables = node.assigned_names(|op| *op == Op::Assign);
	rewrite(node, &Functions { labels, variables })
}

fn is_preposition(word: &str) -> bool {
	PREPOSITIONS.contains(&word)
}

/// `for duration` among a definition's parameters: the preposition and the parameter
fn labelled(parameter: &Node) -> Option<(String, &Node)> {
	match parameter.drop_meta() {
		Node::List(words, Bracket::None, Separator::Space) if words.len() == 2 => match words[0].drop_meta() {
			Node::Symbol(word) if is_preposition(word) => Some((word.clone(), &words[1])),
			_ => None,
		},
		_ => None,
	}
}

/// `f(x, for y) := …`: the name and the parameters of a definition's head (its result type dropped)
fn head_parameters(head: &Node) -> Option<(String, &[Node])> {
	let head = match head.drop_meta() {
		Node::Key(inner, Op::Colon | Op::As, _) => inner.drop_meta(),
		other => other,
	};
	match head {
		Node::List(items, Bracket::Round, _) => match items.first().map(Node::drop_meta) {
			Some(Node::Symbol(name)) => Some((name.clone(), &items[1..])),
			_ => None,
		},
		_ => None,
	}
}

fn functions(node: &Node) -> HashMap<String, Labels> {
	let mut functions: HashMap<String, Labels> = HashMap::new();
	node.visit(&mut |part| {
		let Node::Key(head, Op::Define | Op::Assign, _) = part else { return };
		let Some((name, parameters)) = head_parameters(head) else { return };
		let labels = functions.entry(name).or_default();
		for parameter in parameters {
			let (preposition, parameter) = match labelled(parameter) {
				Some((preposition, parameter)) => (Some(preposition), parameter),
				None => (None, parameter),
			};
			let name = parameter_word(parameter);
			if let Some(preposition) = preposition {
				labels.by_preposition.entry(preposition).or_insert_with(|| name.clone());
			}
			if !labels.parameters.contains(&name) {
				labels.parameters.push(name);
			}
		}
	});
	functions
}

/// `f b=1 a=5`: a call without brackets naming an argument
fn has_spaced_named_call(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| {
		if let Node::List(items, Bracket::None, Separator::Space) = part {
			found |= matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_)))
				&& items[1..].iter().any(|item| super::named_arguments::named_argument(item).is_some());
		}
	});
	found
}

fn rewrite(node: Node, functions: &Functions) -> Node {
	match node {
		Node::Key(head, op @ (Op::Define | Op::Assign), body) if head_parameters(&head).is_some() => {
			key(unlabelled_head(*head), op, rewrite(*body, functions))
		}
		// `(f 9 to 4)` as an argument: the words of a call without brackets
		Node::List(items, bracket @ (Bracket::None | Bracket::Round), Separator::Space) => {
			let items: Vec<Node> = items.into_iter().map(|item| rewrite(item, functions)).collect();
			let words = spread(&items);
			match called(&words, functions) {
				Some(labels) => spaced_call(words, labels, &functions.variables)
					.unwrap_or_else(|| Node::List(items, bracket, Separator::Space)),
				None => Node::List(items, bracket, Separator::Space),
			}
		}
		Node::List(items, Bracket::Round, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| rewrite(item, functions)).collect();
			let items = inner_call(items, functions);
			match called(&items, functions) {
				Some(labels) => {
					let items = items.into_iter().enumerate().map(|(index, item)| match index {
						0 => item,
						_ => renamed(item, labels),
					});
					Node::List(items.collect(), Bracket::Round, separator)
				}
				None => Node::List(items, Bracket::Round, separator),
			}
		}
		// `r = move 1 from 10 to 3` reads `(move 1 from 10) to 3`: the words of the call, when move takes `to`
		Node::Key(left, op @ (Op::To | Op::In), right) => {
			let key = Node::Key(left, op, right);
			let words =
				first_word(&key).and_then(|word| called(std::slice::from_ref(word), functions)).and_then(|labels| {
					let words = spread(&split_operator(key.clone(), labels));
					spaced_call(
						words.into_iter().map(|word| rewrite(word, functions)).collect(),
						labels,
						&functions.variables,
					)
				});
			words.unwrap_or_else(|| key.map_children(|child| rewrite(child, functions)))
		}
		meta @ Node::Meta { .. } => meta.under_meta(|inner| rewrite(inner, functions)),
		other => other.map_children(|child| rewrite(child, functions)),
	}
}

/// `g(f 9 to 4)` holds the words `f 9 to 4` after g: the call of f in it
fn inner_call(mut items: Vec<Node>, functions: &Functions) -> Vec<Node> {
	for start in 1..items.len() {
		if !matches!(items[start].drop_meta(), Node::Symbol(name) if functions.labels.contains_key(name)) {
			continue;
		}
		let words = spread(&items[start..]);
		if let Some(call) =
			called(&words, functions).and_then(|labels| spaced_call(words, labels, &functions.variables))
		{
			items.truncate(start);
			items.push(call);
			break;
		}
	}
	items
}

/// The first word of `(move 1 from 10) to 3`: move
fn first_word(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		Node::Key(left, Op::To | Op::In, _) => first_word(left),
		Node::List(items, Bracket::None, Separator::Space) => first_word(items.first()?),
		_ => Some(node),
	}
}

fn called<'a>(items: &[Node], functions: &'a Functions) -> Option<&'a Labels> {
	match items.first().map(Node::drop_meta) {
		Some(Node::Symbol(name)) => functions.labels.get(name),
		_ => None,
	}
}

/// The head with `for duration` as the parameter `duration`
fn unlabelled_head(head: Node) -> Node {
	match head {
		Node::Key(inner, op @ (Op::Colon | Op::As), result) => key(unlabelled_head(*inner), op, *result),
		Node::List(items, Bracket::Round, separator) => Node::List(
			items
				.into_iter()
				.map(|item| labelled(&item).map_or(item.clone(), |(_, parameter)| parameter.clone()))
				.collect(),
			Bracket::Round,
			separator,
		),
		meta @ Node::Meta { .. } => meta.under_meta(unlabelled_head),
		other => other,
	}
}

/// `for: 0.5s` of a function passing `for` to duration: `duration: 0.5s`
fn renamed(argument: Node, labels: &Labels) -> Node {
	match super::named_arguments::named_argument(&argument) {
		Some((name, value)) if labels.by_preposition.contains_key(&name) => {
			key(Node::Symbol(labels.by_preposition[&name].clone()), Op::Colon, value.clone())
		}
		_ => argument,
	}
}

/// `r = f 9 from 4` groups `[f 9] [from 4]`: the words, groups without brackets spread
fn spread(items: &[Node]) -> Vec<Node> {
	items
		.iter()
		.flat_map(|item| match item.drop_meta() {
			Node::List(inner, Bracket::None, Separator::Space) => spread(inner),
			_ => vec![item.clone()],
		})
		.collect()
}

/// `play 440Hz for 0.5s`, `f b=1 a=5`: the call `play(440Hz, duration: 0.5s)`, `f(b=1, a=5)`; None for words without
/// prepositions or named arguments, which stay as written for the passes reading calls without brackets
fn spaced_call(items: Vec<Node>, labels: &Labels, variables: &HashSet<String>) -> Option<Node> {
	let words: Vec<Node> = items.into_iter().flat_map(|item| split_operator(item, labels)).collect();
	if let Some(foreign) = words[1..].iter().find(|item| is_foreign_preposition(item, labels, variables)) {
		return Some(unknown_preposition(&words[0], foreign, labels));
	}
	let is_label = |item: &Node| is_label_of(item, labels);
	let names_argument = |item: &Node| {
		super::named_arguments::named_argument(item)
			.is_some_and(|(name, _)| labels.parameters.contains(&name) || labels.by_preposition.contains_key(&name))
	};
	if !words[1..].iter().any(|item| is_label(item) || names_argument(item)) {
		return None;
	}
	let mut arguments = vec![words[0].clone()];
	let mut rest = words.into_iter().skip(1);
	while let Some(item) = rest.next() {
		if is_label(&item) {
			match rest.next() {
				Some(value) => {
					arguments.push(key(Node::Symbol(labels.by_preposition[&item.name()].clone()), Op::Colon, value))
				}
				None => return Some(missing_value(&item)),
			}
		} else {
			arguments.push(renamed(item, labels));
		}
	}
	Some(parens(arguments))
}

fn is_label_of(item: &Node, labels: &Labels) -> bool {
	matches!(item.drop_meta(), Node::Symbol(word) if labels.by_preposition.contains_key(word))
}

/// `move 1 from 10 to 3` parses `10 to 3` as a range: in a call of a function passing `to`, the words as written
fn split_operator(item: Node, labels: &Labels) -> Vec<Node> {
	let word = match item.drop_meta() {
		Node::Key(_, Op::To, _) => "to",
		Node::Key(_, Op::In, _) => "in",
		// `f 3 in 4` holds the words `3 in 4` as a group of their own
		Node::List(inner, Bracket::None, Separator::Space)
			if spread(inner).iter().any(|word| is_label_of(word, labels)) =>
		{
			return spread(inner).into_iter().flat_map(|word| split_operator(word, labels)).collect()
		}
		_ => return vec![item],
	};
	if !labels.by_preposition.contains_key(word) {
		return vec![item];
	}
	let Node::Key(left, _, right) = item.drop_meta().clone() else { unreachable!("a key") };
	let mut words = split_operator(*left, labels);
	words.push(Node::Symbol(word.into()));
	words.extend(split_operator(*right, labels));
	words
}

/// `f 5 by 2` of `f(x, with y)`: a preposition the function does not take
fn is_foreign_preposition(item: &Node, labels: &Labels, variables: &HashSet<String>) -> bool {
	!labels.by_preposition.is_empty()
		&& matches!(item.drop_meta(), Node::Symbol(word) if is_preposition(word) && !labels.by_preposition.contains_key(word) && !variables.contains(word))
}

fn unknown_preposition(function: &Node, preposition: &Node, labels: &Labels) -> Node {
	let mut taken: Vec<&String> = labels.by_preposition.keys().collect();
	taken.sort();
	let taken = taken.iter().map(|word| word.as_str()).collect::<Vec<_>>().join(", ");
	let message =
		format!("{} takes no argument after {}; it takes one after {taken}", function.name(), preposition.name());
	Diagnostic::at(preposition, message).into_error()
}

fn missing_value(preposition: &Node) -> Node {
	let word = preposition.name();
	Diagnostic::at(preposition, format!("{word} needs a value after it: `{word} 1s`")).into_error()
}
