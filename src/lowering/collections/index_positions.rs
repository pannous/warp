//! Card index-position: `index` and `position` are ints that know where they count from. An index counts from 0
//! (`xs[i]`), a position from 1 (`xs#p`). `xs.index_of(x)` gives a position, `argmax(xs)` and `argmin(xs)` an index,
//! `xs.insert(v, at: i)` takes an index, and a variable may be declared either (`p: position`, `i: index`).
//! Where one meets the other's brackets it converts: `xs[p]` is `xs#p`, `xs#i` is `xs[i]`, `at: p` is `at: p - 1`.
//! Arithmetic keeps the dimension (user decision), as size_t or a pointer does: `p + 1` is the next position, `i - 1`
//! the index before; `q - p` of two positions is a plain count, and adding two of them is an error. Printing is the
//! plain number.
//! A name counts as a position (an index) when it is declared one, or when every value assigned to it is one.
//! Design: notes/index_position.md

use super::memoization::definition_parts;
use super::nodes::{children_rewritten, for_in_variable, key, parameter_name};
use crate::errors::diagnostic::Diagnostic;
use crate::library_words::COLLECTION_POSITION;
use crate::list_mutations::{insert_position, INSERT_METHOD};
use crate::node::{parens, Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::SubscriptOffset;
use crate::words::AT_WORD;
use std::collections::{HashMap, HashSet};

const POSITION_TYPE: &str = "position";
const INDEX_TYPE: &str = "index";
/// The type a declared position or index is checked as
const COUNT_TYPE: &str = "int";
const INDEX_CALLS: [&str; 2] = ["argmax", "argmin"];
const POSITION_CALLS: [&str; 1] = [COLLECTION_POSITION];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Count {
	Position,
	Index,
}

impl Count {
	fn of_type_word(word: &str) -> Option<Count> {
		match word {
			POSITION_TYPE => Some(Count::Position),
			INDEX_TYPE => Some(Count::Index),
			_ => None,
		}
	}
}

/// The mark a declared `p: position` keeps on its annotation, read `p: int` by every other pass
#[derive(Clone, Debug, PartialEq)]
struct DeclaredCount(Count);

/// `p: position`, `i: index` as `p: int` marked with what it counts, before the passes that check declarations; a
/// program that binds `position` or `index` as a name keeps the word as it is
pub fn mark_declarations(node: Node) -> Node {
	if !node.mentions_any(&[POSITION_TYPE, INDEX_TYPE]) {
		return node;
	}
	let bound = assigned_or_declared_names(&node);
	let free_type_words: Vec<&str> =
		[POSITION_TYPE, INDEX_TYPE].into_iter().filter(|word| !bound.contains(*word)).collect();
	marked_declarations(node, &free_type_words)
}

fn marked_declarations(node: Node, type_words: &[&str]) -> Node {
	if let Node::Key(name, Op::Colon, annotation) = node.drop_meta() {
		let counted = annotation.symbol_name().filter(|word| type_words.contains(word));
		if let (Some(_), Some(count)) = (name.symbol_name(), counted.and_then(Count::of_type_word)) {
			let marked = Node::meta(Node::Symbol(COUNT_TYPE.to_string()), Node::data(DeclaredCount(count)));
			return key(name.as_ref().clone(), Op::Colon, marked);
		}
	}
	children_rewritten(node, |child| marked_declarations(child, type_words))
}

/// Every conversion where a position or an index meets the other's brackets, after library_words names index_of
/// collection_position
pub fn lower(node: Node) -> Node {
	let counted = scope_counts(&node, vec![], &HashMap::new());
	converted(node, &counted)
}

fn converted(node: Node, counted: &HashMap<String, Count>) -> Node {
	if !matches!(node, Node::Meta { .. }) {
		if let Some((head, body, rebuild)) = definition_parts(&node) {
			let parameters = head.children();
			let mut bindings: Vec<Binding> =
				parameters.iter().skip(1).filter_map(parameter_name).map(Binding::Plain).collect();
			collect_bindings(&head, &mut bindings);
			let own = scope_counts(&body, bindings, counted);
			return rebuild(head, converted(body, &own));
		}
	}
	let node = children_rewritten(node, |child| converted(child, counted));
	match node {
		Node::Key(target, Op::Hash, place) => key(*target, Op::Hash, hash_place(*place, counted)),
		Node::Key(left, Op::Add, right)
			if count_of(&left, counted).is_some() && count_of(&right, counted).is_some() =>
		{
			let sum = key(*left, Op::Add, *right);
			let message = format!("{} adds two positions or indexes: subtract them for a count", sum.serialize());
			Diagnostic::at(&sum, message).into_error()
		}
		Node::List(items, Bracket::Round, Separator::None) if is_call(&items, INSERT_METHOD) => {
			let items = items.into_iter().map(|item| at_index(item, counted)).collect();
			parens(items)
		}
		other => other,
	}
}

/// The argument `at: p` of `xs.insert(v, at: p)` with a position p: `at: p - 1`
fn at_index(argument: Node, counted: &HashMap<String, Count>) -> Node {
	match insert_position(&argument) {
		Some(position) if count_of(position, counted) == Some(Count::Position) => {
			let index = key(position.clone(), Op::Sub, Node::int(1));
			key(Node::Symbol(AT_WORD.to_string()), Op::Colon, index)
		}
		_ => argument,
	}
}

/// The place of `xs#place`: a position under the subscript's offset is the place itself, an index the place after it
fn hash_place(place: Node, counted: &HashMap<String, Count>) -> Node {
	if let Some(subscripted) = subscripted_index(&place) {
		if count_of(subscripted, counted) == Some(Count::Position) {
			return subscripted.clone();
		}
		return place;
	}
	match count_of(&place, counted) {
		Some(Count::Index) => key(place, Op::Add, Node::int(1)),
		_ => place,
	}
}

/// `i` of `i + 1` when the `+ 1` is the one `xs[i]` adds (warp_parser subscript), not one written by hand
fn subscripted_index(place: &Node) -> Option<&Node> {
	match place.drop_meta() {
		Node::Key(index, Op::Add, one) if is_subscript_offset(one) => Some(index),
		_ => None,
	}
}

fn is_subscript_offset(node: &Node) -> bool {
	match node {
		Node::Meta { node, data } => {
			matches!(data.as_ref(), Node::Data(dada) if dada.downcast_ref::<SubscriptOffset>().is_some())
				|| is_subscript_offset(node)
		}
		_ => false,
	}
}

fn count_of(node: &Node, counted: &HashMap<String, Count>) -> Option<Count> {
	match node.drop_meta() {
		Node::Symbol(name) => counted.get(name).copied(),
		Node::List(items, Bracket::Round, Separator::None) if POSITION_CALLS.iter().any(|c| is_call(items, c)) => {
			Some(Count::Position)
		}
		Node::List(items, Bracket::Round, Separator::None) if INDEX_CALLS.iter().any(|c| is_call(items, c)) => {
			Some(Count::Index)
		}
		Node::List(items, _, _) if items.len() == 1 => count_of(&items[0], counted),
		Node::Key(left, op @ (Op::Add | Op::Sub), right) => {
			match (count_of(left, counted), count_of(right, counted), op) {
				(Some(count), None, _) | (None, Some(count), Op::Add) => Some(count),
				_ => None,
			}
		}
		_ => None,
	}
}

fn is_call(items: &[Node], function: &str) -> bool {
	items.first().is_some_and(|head| head.is_symbol(function))
}

/// What the names of one scope count: those of the scope around it it doesn't bind again, and its own, declared a
/// position or an index or assigned only such values; a function defined inside is a scope of its own
fn scope_counts(body: &Node, mut bindings: Vec<Binding>, outer: &HashMap<String, Count>) -> HashMap<String, Count> {
	collect_bindings(body, &mut bindings);
	let mut own: HashMap<String, Option<Count>> = HashMap::new();
	for binding in &bindings {
		if let Binding::Declared(name, count) = binding {
			own.insert(name.clone(), Some(*count));
		}
	}
	let declared = own.clone();
	for binding in bindings {
		let (name, count) = match binding {
			Binding::Declared(..) => continue,
			Binding::Assigned(name, value) => {
				let count = count_of(&value, &counted(&declared)).or_else(|| count_of(&value, outer));
				(name, count)
			}
			Binding::Plain(name) => (name, None),
		};
		if declared.contains_key(&name) {
			continue;
		}
		own.entry(name)
			.and_modify(|known| {
				if *known != count {
					*known = None
				}
			})
			.or_insert(count);
	}
	let inherited = outer.iter().filter(|(name, _)| !own.contains_key(*name));
	inherited.map(|(name, count)| (name.clone(), *count)).chain(counted(&own)).collect()
}

fn counted(names: &HashMap<String, Option<Count>>) -> HashMap<String, Count> {
	names.iter().filter_map(|(name, count)| Some((name.clone(), (*count)?))).collect()
}

enum Binding {
	Declared(String, Count),
	Assigned(String, Node),
	/// a loop variable, or a name declared some other type
	Plain(String),
}

/// The names one scope binds, not those of the functions it defines
fn collect_bindings(node: &Node, bindings: &mut Vec<Binding>) {
	if !matches!(node, Node::Meta { .. }) && definition_parts(node).is_some() {
		return;
	}
	match node.drop_meta() {
		Node::Key(name, Op::Colon, annotation) => {
			if let Some(name) = name.symbol_name() {
				bindings.push(match declared_count(annotation) {
					Some(count) => Binding::Declared(name.to_string(), count),
					None if annotation.symbol_name().is_some() => Binding::Plain(name.to_string()),
					None => return collect_parts(node, bindings),
				});
			}
		}
		Node::Key(name, Op::Assign, value) => {
			if let Some(name) = name.symbol_name() {
				bindings.push(Binding::Assigned(name.to_string(), value.as_ref().clone()));
			}
		}
		Node::List(items, _, _) => {
			if let Some(variable) = for_in_variable(items) {
				bindings.push(Binding::Plain(variable));
			}
		}
		_ => {}
	}
	collect_parts(node, bindings);
}

fn collect_parts(node: &Node, bindings: &mut Vec<Binding>) {
	node.drop_meta().parts().into_iter().for_each(|part| collect_bindings(part, bindings));
}

fn declared_count(annotation: &Node) -> Option<Count> {
	match annotation {
		Node::Meta { node, data } => match data.as_ref() {
			Node::Data(dada) => {
				dada.downcast_ref::<DeclaredCount>().map(|DeclaredCount(count)| *count).or_else(|| declared_count(node))
			}
			_ => declared_count(node),
		},
		_ => None,
	}
}

/// `name = value` anywhere in the program
fn assignments(node: &Node) -> Vec<(String, Node)> {
	let mut found = vec![];
	node.visit(&mut |node| {
		if let Node::Key(name, Op::Assign, value) = node.drop_meta() {
			if let Some(name) = name.symbol_name() {
				found.push((name.to_string(), value.as_ref().clone()));
			}
		}
	});
	found
}

/// The names a program assigns or declares
fn assigned_or_declared_names(node: &Node) -> HashSet<String> {
	let declared: Vec<String> = node.assigned_names(|op| *op == Op::Colon);
	assignments(node).into_iter().map(|(name, _)| name).chain(declared).collect()
}
