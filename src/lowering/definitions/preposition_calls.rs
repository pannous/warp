//! Prepositions name arguments (card prepositions-call, user 2026-10-14): a parameter written after a preposition,
//! `play(tone, for duration)`, is passed by that word as Swift passes its argument labels, always with a colon (decision
//! labels-calls): `play 440Hz for: 0.5s`, `play(440Hz, for: 0.5s)` and `play tone:440Hz for:0.5s` are all
//! `play(440Hz, duration: 0.5s)`, which named_arguments.rs orders. A call without brackets also takes named arguments,
//! `f b=1 a=5` is `f(b=1, a=5)`. Without its colon the word keeps its own meaning, `for x in xs` a loop, `1 to 5` a
//! range (P52); in a call of a function it labels, the bare word is an error naming the colon form, a warning where it
//! cannot mean anything else (UNAMBIGUOUS_LABELS).

use super::nodes::{any_part, key, parameter_word};
use crate::errors::diagnostic::Diagnostic;
use crate::node::{parens, Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_name_matching::{is_label_word, AS_LABEL, PREPOSITIONS};
use std::collections::{HashMap, HashSet};

/// The labels whose bare form after a call's argument means nothing else: `play C4 for 1s` is read as `for: 1s` with a
/// warning naming the colon (user 2026-10-15). `to` is a range, `in` a membership, `as` a cast, `of` a field, and
/// with/from/into/by get a meaning of their own soon (decision labels-calls): those stay errors
const UNAMBIGUOUS_LABELS: [&str; 2] = ["for", "each"];

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
	if !node.mentions_any(&PREPOSITIONS) && !node.mentions(AS_LABEL) && !has_spaced_named_call(&node) {
		return node;
	}
	let labels = functions(&node);
	if labels.is_empty() {
		return node;
	}
	let variables = node.assigned_names(|op| *op == Op::Assign);
	rewrite(node, &Functions { labels, variables })
}

/// `for duration` among a definition's parameters: the preposition and the parameter
fn labelled(parameter: &Node) -> Option<(String, &Node)> {
	match parameter.drop_meta() {
		Node::List(words, Bracket::None, Separator::Space) if words.len() == 2 && is_label_word(&words[0]) => {
			Some((words[0].name(), &words[1]))
		}
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
	any_part(
		node,
		|part| matches!(part, Node::List(items, Bracket::None, Separator::Space) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) && items[1..].iter().any(|item| super::named_arguments::named_argument(item).is_some())),
	)
}

fn rewrite(node: Node, functions: &Functions) -> Node {
	match node {
		Node::Key(head, op @ (Op::Define | Op::Assign), body) if head_parameters(&head).is_some() => {
			key(unlabelled_head(*head), op, rewrite(*body, functions))
		}
		// `(f 9 to: 4)` as an argument: the words of a call without brackets; `r = move 1 to: 3 from: 10` groups
		// `[[move 1 to: 3] from: 10]`, spread before any is a call; `{melody [C3 G2] each: 1 beat}` stays a block
		Node::List(items, bracket, separator) if crate::lowering::nodes::is_words_call(&bracket, &separator) => {
			let words = spread(&items);
			let call = called(&words, functions).and_then(|labels| {
				let words = words.iter().map(|word| rewrite(word.clone(), functions)).collect();
				spaced_call(words, labels, &functions.variables)
			});
			match call {
				Some(call) => crate::lowering::nodes::in_block_as_written(&bracket, call),
				None => {
					Node::List(items.into_iter().map(|item| rewrite(item, functions)).collect(), bracket, separator)
				}
			}
		}
		Node::List(items, Bracket::Round, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| rewrite(item, functions)).collect();
			let items = inner_call(items, functions);
			match called(&items, functions) {
				Some(labels) => {
					if let Some(error) = items[1..].iter().find_map(|item| label_error(&items[0], item, labels)) {
						return error;
					}
					let items = items.into_iter().enumerate().map(|(index, item)| match index {
						0 => item,
						_ => renamed(item, labels),
					});
					Node::List(items.collect(), Bracket::Round, separator)
				}
				None => Node::List(items, Bracket::Round, separator),
			}
		}
		meta @ Node::Meta { .. } => meta.under_meta(|inner| rewrite(inner, functions)),
		other => other.map_children(|child| rewrite(child, functions)),
	}
}

/// `g(f 9 to: 4)` holds the words `f 9 to: 4` after g: the call of f in it
fn inner_call(mut items: Vec<Node>, functions: &Functions) -> Vec<Node> {
	for start in 1..items.len() {
		if !matches!(items[start].drop_meta(), Node::Symbol(name) if functions.labels.contains_key(name)) {
			continue;
		}
		let words = spread(&items[start..]);
		// `play(hum, for: 0.3s)`: a label hum does not take is the outer call's
		let takes_the_labels =
			|labels: &&Labels| words[1..].iter().all(|word| label_error(&words[0], word, labels).is_none());
		let labels = called(&words, functions).filter(takes_the_labels);
		if let Some(call) = labels.and_then(|labels| spaced_call(words, labels, &functions.variables)) {
			items.truncate(start);
			items.push(call);
			break;
		}
	}
	items
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

/// `r = f 9 from: 4` groups `[f 9] [from: 4]`: the words, groups without brackets spread
fn spread(items: &[Node]) -> Vec<Node> {
	items
		.iter()
		.flat_map(|item| match item.drop_meta() {
			Node::List(inner, Bracket::None, Separator::Space) => spread(inner),
			_ => vec![item.clone()],
		})
		.collect()
}

/// `play 440Hz for: 0.5s`, `f b=1 a=5`: the call `play(440Hz, duration: 0.5s)`, `f(b=1, a=5)`; None for words without
/// labels or named arguments, which stay as written for the passes reading calls without brackets
fn spaced_call(words: Vec<Node>, labels: &Labels, variables: &HashSet<String>) -> Option<Node> {
	let words = with_unambiguous_colons(words, labels, variables);
	if let Some(error) = label_without_colon(&words, labels, variables) {
		return Some(error);
	}
	if let Some(error) = words[1..].iter().find_map(|item| label_error(&words[0], item, labels)) {
		return Some(error);
	}
	let names_argument = |item: &Node| {
		super::named_arguments::named_argument(item)
			.is_some_and(|(name, _)| labels.parameters.contains(&name) || labels.by_preposition.contains_key(&name))
	};
	if !words[1..].iter().any(names_argument) {
		return None;
	}
	let arguments = words.into_iter().enumerate().map(|(index, item)| match index {
		0 => item,
		_ => renamed(item, labels),
	});
	Some(parens(arguments.collect()))
}

/// `play C4 for 1s`: `play C4 for: 1s`, said as a warning offering the colon (UNAMBIGUOUS_LABELS)
fn with_unambiguous_colons(words: Vec<Node>, labels: &Labels, variables: &HashSet<String>) -> Vec<Node> {
	let mut supplied: Vec<Node> = vec![];
	let mut words = words.into_iter();
	while let Some(word) = words.next() {
		let label = match word.drop_meta() {
			Node::Symbol(label) if UNAMBIGUOUS_LABELS.contains(&label.as_str()) && !supplied.is_empty() => {
				label.clone()
			}
			_ => {
				supplied.push(word);
				continue;
			}
		};
		if variables.contains(&label) || !labels.by_preposition.contains_key(&label) {
			supplied.push(word);
			continue;
		}
		let Some(value) = words.next() else {
			supplied.push(word);
			continue;
		};
		warn_missing_colon(&supplied[0], &word, &value);
		supplied.push(key(Node::Symbol(label), Op::Colon, value));
	}
	supplied
}

/// The warning on `play C4 for 1s`, offering `for: 1s`
pub(crate) fn warn_missing_colon(function: &Node, label: &Node, value: &Node) {
	let word = label.drop_meta().name();
	crate::errors::diagnostic::say(missing_colon(function, label, &word, Some(value)));
}

/// `move 1 from 2 to: 3`: the label `from` without its colon, offering `from: 2` when its value follows
fn missing_colon(function: &Node, label: &Node, word: &str, value: Option<&Node>) -> Diagnostic {
	let message = format!("{word} labels an argument of {} with a colon: `{word}: …`", function.name());
	let diagnostic = Diagnostic::at(label, message);
	match value.map(|value| value.serialize().trim().to_string()) {
		Some(value) => diagnostic.offer("the label's colon", format!("{word} {value}"), format!("{word}: {value}")),
		None => diagnostic,
	}
}

/// `play C4 for 1s`, `save sound as "x.wav"`, `move x to 5` of functions taking those labels: a label without its
/// colon (a variable of that name is an argument)
fn bare_label<'a>(item: &'a Node, labels: &Labels, variables: &HashSet<String>) -> Option<&'a str> {
	let word = match item.drop_meta() {
		Node::Symbol(word) if !variables.contains(word) => word.as_str(),
		Node::Key(_, Op::To, _) => "to",
		Node::Key(_, Op::In, _) => "in",
		Node::Key(_, Op::As, _) => AS_LABEL,
		_ => return None,
	};
	labels.by_preposition.contains_key(word).then_some(word)
}

/// The error on the first bare label of a call's words: `from 2` after a symbol, `to 3` of the key `2 to 3`
fn label_without_colon(words: &[Node], labels: &Labels, variables: &HashSet<String>) -> Option<Node> {
	let (index, word) =
		(1..words.len()).find_map(|index| Some((index, bare_label(&words[index], labels, variables)?)))?;
	let value = match words[index].drop_meta() {
		Node::Key(_, _, right) => Some(right.as_ref()),
		_ => words.get(index + 1),
	};
	Some(missing_colon(&words[0], &words[index], word, value).into_error())
}

/// `f 5 by: 2` of `f(x, with y)`, `f 5 for:`: a label the function does not take, a label without its value
fn label_error(function: &Node, item: &Node, labels: &Labels) -> Option<Node> {
	let (name, value) = super::named_arguments::named_argument(item)?;
	if !is_label_word(&Node::Symbol(name.clone())) || labels.parameters.contains(&name) {
		return None;
	}
	if !labels.by_preposition.contains_key(&name) {
		let mut taken: Vec<String> = labels.by_preposition.keys().map(|word| format!("{word}:")).collect();
		taken.sort();
		let message = match taken.is_empty() {
			true => format!("{} takes no labelled argument, not {name}:", function.name()),
			false => format!("{} takes no argument labelled {name}:; it takes {}", function.name(), taken.join(", ")),
		};
		return Some(Diagnostic::at(item, message).into_error());
	}
	value
		.is_empty_node()
		.then(|| Diagnostic::at(item, format!("{name}: needs a value after it: `{name}: 1s`")).into_error())
}
