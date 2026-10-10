//! take, zip, list and sum over generators (card generators-zip): `take(naturals(), 3)` and `zip(naturals(), xs)` pull
//! only the values they need, so an endless generator works, and `take(n, 2)` of a generator object goes on where the
//! last `next(n)` stopped; `list(c)` and `sum(c)` of an object take the rest of its values. Each such call becomes the
//! statements before its statement that pull the values one at a time: from a generator call through its object
//! (`iter(g(args))`, generator_objects.rs), from an object by `.next()` until ø, from any other list by its index.
//! A call inside a `while` condition stays as it is: computed once before the loop it would not change per test.

use super::words::OF_WORD;
use crate::generator_objects::{advanced_variables, generator_call, ITER_WORD};
use crate::generators::{is_statement_list, yielded_value, Generator, NAME_SEPARATOR};
use super::nodes::{Counter, assign, block, call, int, statement_list, symbol};
use crate::library_words::substitute;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::{parse, while_do};
use std::collections::{HashMap, HashSet};

const TAKE_WORD: &str = "take";
/// `first(xs, 3)` and the phrases `take 3 of xs`, `first 3 of xs` are take too
const TAKE_WORDS: [&str; 2] = [TAKE_WORD, "first"];
const ZIP_WORD: &str = "zip";
const LIST_WORD: &str = "list";
/// Calls that read the rest of an object's values as a list, under any of their spellings (`average`, `sorted`)
const DRAINING_WORDS: [&str; 7] = [LIST_WORD, "sum", "count", "max", "min", "mean", "sort"];
const PULLED: &str = "VALUE = SOURCE.next(); if VALUE == ø { break }";
const INDEXED: &str = "INDEX += 1; if INDEX > count(SOURCE) { break }; VALUE = SOURCE#INDEX";
const EMPTY: &str = "OUT = []";
const COLLECTED: &str = "OUT += [VALUE]";
const TAKEN_ENOUGH: &str = "count(OUT) < LIMIT";

/// Where a consumer gets its values from
enum Source {
	/// An object with `next()`, ø at its end
	Pulled(Node),
	/// A list, by index
	Indexed(Node),
}

struct Consumers<'a> {
	generators: &'a HashMap<String, Generator>,
	/// Variables holding an object with `next()`
	objects: HashSet<String>,
	counter: Counter,
}

pub(crate) fn lower(node: Node, generators: &HashMap<String, Generator>) -> Node {
	let mut objects = advanced_variables(&node);
	objects.extend(lazily_read_variables(&node, generators));
	let consumers = Consumers { generators, objects, counter: Counter::starting_at(1) };
	let mut found = false;
	node.visit(&mut |part| found |= consumers.consumer(part).is_some());
	if !found {
		return node;
	}
	let node = match node.drop_meta() {
		Node::List(_, Bracket::None | Bracket::Curly, Separator::Semicolon | Separator::Newline) => node,
		_ => statement_list(vec![node], Bracket::None),
	};
	consumers.hoisted(node)
}

/// `n` of `n = naturals()` given to take or zip: it becomes an object, so an endless generator is not collected first
fn lazily_read_variables(node: &Node, generators: &HashMap<String, Generator>) -> HashSet<String> {
	let mut assigned = HashSet::new();
	let mut read = HashSet::new();
	node.visit(&mut |part| {
		if let Node::Key(target, Op::Assign, value) = part.drop_meta() {
			if generator_call(value, generators).is_some() {
				assigned.extend(target.symbol_name().map(String::from));
			}
		}
		if let Some((word, arguments)) = consumer_call(part) {
			if word == TAKE_WORD || word == ZIP_WORD {
				read.extend(arguments.iter().filter_map(Node::symbol_name).map(String::from));
			}
		}
	});
	assigned.intersection(&read).cloned().collect()
}

/// `word(arguments)`, `take 3 of xs` as `take(xs, 3)`
fn consumer_call(node: &Node) -> Option<(&str, Vec<Node>)> {
	let Node::List(items, Bracket::Round | Bracket::None, _) = node.drop_meta() else { return None };
	let is_take = |word: &Node| word.symbol_name().is_some_and(|name| TAKE_WORDS.contains(&name));
	if let [word, count, of, source] = items.as_slice() {
		if is_take(word) && of.is_symbol(OF_WORD) {
			return Some((TAKE_WORD, vec![source.clone(), count.clone()]));
		}
	}
	let (name, arguments) = items.split_first()?;
	let arguments: Vec<Node> = arguments.iter().flat_map(crate::ruby_blocks::arguments).collect();
	let word = if is_take(name) && arguments.len() == 2 { TAKE_WORD } else { name.symbol_name()? };
	Some((word, arguments))
}

fn is_while(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(head, Op::Do, _) if matches!(head.drop_meta(), Node::Key(_, Op::While, _)))
}

/// The statements of a template with its placeholders filled in
pub(crate) fn template(text: &str, bindings: &[(&str, &Node)]) -> Vec<Node> {
	let filled = bindings.iter().fold(parse(text), |node, (placeholder, value)| substitute(node, placeholder, value));
	match filled.drop_meta() {
		Node::List(items, Bracket::None, Separator::Semicolon | Separator::Newline) => items.clone(),
		single => vec![single.clone()],
	}
}

impl Consumers<'_> {
	/// A consumer call: its word, its sources and, for take, the count
	fn consumer(&self, node: &Node) -> Option<(String, Vec<Source>, Option<Node>)> {
		let (word, mut arguments) = consumer_call(node)?;
		let limit = match word {
			TAKE_WORD if arguments.len() == 2 => arguments.pop(),
			ZIP_WORD if arguments.len() >= 2 => None,
			_ if DRAINING_WORDS.contains(&crate::library_words::canonical_spelling(word)) && arguments.len() == 1 && arguments[0].symbol_name().is_some_and(|name| self.objects.contains(name)) => None,
			_ => return None,
		};
		let sources: Vec<Source> = arguments.into_iter().map(|argument| self.source(argument)).collect();
		sources.iter().any(|source| matches!(source, Source::Pulled(_))).then(|| (word.to_string(), sources, limit))
	}

	fn source(&self, argument: Node) -> Source {
		let is_object = argument.symbol_name().is_some_and(|name| self.objects.contains(name));
		if is_object {
			Source::Pulled(argument)
		} else if generator_call(&argument, self.generators).is_some() {
			Source::Pulled(template(&format!("{ITER_WORD}(CALL)"), &[("CALL", &argument)]).remove(0))
		} else {
			Source::Indexed(argument)
		}
	}

	/// Each statement list with the consumers of its statements computed before them
	fn hoisted(&self, node: Node) -> Node {
		match node {
			Node::List(items, bracket, separator) if is_statement_list(&items, &bracket, &separator) => {
				let separator = if separator == Separator::Newline { separator } else { Separator::Semicolon };
				let items = items.into_iter().flat_map(|item| {
					let item = self.hoisted(item);
					if is_while(&item) {
						return vec![item];
					}
					let mut before = vec![];
					let item = self.extracted(item, &mut before);
					before.push(item);
					before
				});
				Node::List(items.collect(), bracket, separator)
			}
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.hoisted(*node)), data },
			other => other.map_children(|child| self.hoisted(child)),
		}
	}

	/// The statement with each consumer replaced by the variable its statements, put into `before`, compute
	fn extracted(&self, node: Node, before: &mut Vec<Node>) -> Node {
		if matches!(node.drop_meta(), Node::List(items, bracket, separator) if is_statement_list(items, bracket, separator)) || matches!(node.drop_meta(), Node::Key(_, Op::Define | Op::FatArrow | Op::Arrow, _)) {
			return node;
		}
		let node = node.map_children(|child| self.extracted(child, before));
		let Some((word, sources, limit)) = self.consumer(&node) else { return node };
		let number = self.counter.next_number().to_string();
		let name = |parts: &[&str]| symbol(&[&[word.as_str(), &number], parts].concat().join(NAME_SEPARATOR));
		let out = name(&[]);
		before.extend(template(EMPTY, &[("OUT", &out)]));
		let mut condition = int(1);
		if let Some(limit) = limit {
			let limit_name = name(&["limit"]);
			before.push(assign(limit_name.clone(), limit));
			condition = template(TAKEN_ENOUGH, &[("OUT", &out), ("LIMIT", &limit_name)]).remove(0);
		}
		let mut body = vec![];
		let mut values = vec![];
		for (index, source) in sources.into_iter().enumerate() {
			let value = name(&["value", &index.to_string()]);
			let source_name = name(&["source", &index.to_string()]);
			let (expression, pull) = match source {
				Source::Pulled(expression) => (expression, PULLED),
				Source::Indexed(expression) => {
					before.push(assign(name(&["index", &index.to_string()]), int(0)));
					(expression, INDEXED)
				}
			};
			let pulled_from = if expression.symbol_name().is_some() && pull == PULLED { expression } else {
				before.push(assign(source_name.clone(), expression));
				source_name
			};
			body.extend(template(pull, &[("VALUE", &value), ("SOURCE", &pulled_from), ("INDEX", &name(&["index", &index.to_string()]))]));
			values.push(value);
		}
		body.extend(template(COLLECTED, &[("OUT", &out), ("VALUE", &yielded_value(values))]));
		before.push(while_do(condition, block(body)));
		match word.as_str() {
			TAKE_WORD | ZIP_WORD | LIST_WORD => out,
			_ => call(&word, vec![out]),
		}
	}
}
