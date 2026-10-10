//! A number subscript on a variable that starts as the empty map `{}` keys it, like a text subscript (user decision
//! P34): `d={}; d[1]="a"` is {1:"a"} and `d[1]` reads that entry. On any other value `d[1]` stays a position.
//! A number known only at run time keys it too, by its digits: `d[k]` is `d[string(k)]`, and so is a membership test
//! `k in d`, `d has k`, `d.remove(k)` and `d.get(k)` (lower_key_words, after library_words names them), so a map keyed
//! by ids stays a hash table.
//! A map literal with number keys is such a map too (card map-literal): `m = {1: 5}` stores the key "1", so `m[1] = 2`
//! sets that entry and `m["1"]` reads it.
//! A function definition is its own scope (card map-locals): its `out = {}` makes only its own `out` such a map, a
//! name it assigns or takes as a parameter is its own, and a map of main stays one where the function only reads it.

use super::memoization::definition_parts;
use super::nodes::{call, parameter_name};
use crate::library_words::{COLLECTION_CONTAINS, COLLECTION_POSITION, MAP_GET_OR, MAP_WITHOUT};
use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::warp_parser::WrittenIndex;
use std::collections::HashSet;

/// The word that makes a key of a number known only at run time
const KEY_TEXT: &str = "string";
/// The declared types that make a variable a map (declarations.rs: dict is another language's word for map)
const MAP_TYPE_WORDS: [&str; 2] = ["map", "dict"];
/// The words `word(map, key, …)` that find an entry by its key
const KEY_WORDS: [&str; 5] = [COLLECTION_CONTAINS, COLLECTION_POSITION, MAP_GET_OR, MAP_WITHOUT, crate::analyzer::REMOVED_VALUE_CALL];

pub fn lower(node: Node) -> Node {
	if !has_map_variable(&node) {
		return node;
	}
	let maps = map_variables(&node);
	keyed(node, &maps)
}

/// `collection_contains(d, k)`, `map_without(d, k)` … (KEY_WORDS) of a map `d` that starts as `{}` find the key
/// `string(k)`; the number keys of its literal `{1: 5}` become the texts they are found by. Runs after `lower`, which
/// still sees the number keys that make a literal a map.
pub fn lower_key_words(node: Node) -> Node {
	if !has_map_variable(&node) {
		return node;
	}
	let maps = map_variables(&node);
	found_by_key(node, &maps)
}

/// A variable assigned the empty map `{}` or a literal with number keys `{1: 5}`, also when declared a map
/// (`m: map<int, int> = {}`, card typed-map)
fn map_assigned(part: &Node) -> Option<&str> {
	match part.drop_meta() {
		Node::Key(target, Op::Assign, value) if is_number_keyed(value) => assigned_name(target),
		_ => None,
	}
}

fn has_map_variable(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= map_assigned(part).is_some());
	found
}

/// The map variables of one scope, not those of the functions it defines
fn map_variables(node: &Node) -> HashSet<String> {
	let mut maps = HashSet::new();
	collect_map_variables(node, &mut maps);
	maps
}

fn collect_map_variables(node: &Node, maps: &mut HashSet<String>) {
	if definition_parts(node).is_some() {
		return;
	}
	maps.extend(map_assigned(node).map(str::to_string));
	node.drop_meta().parts().into_iter().for_each(|part| collect_map_variables(part, maps));
}

/// A function definition with its body rewritten for the maps it sees: those of the scope around it it doesn't
/// take or assign itself, and its own
fn in_function_scope(node: &Node, maps: &HashSet<String>, rewrite: fn(Node, &HashSet<String>) -> Node) -> Option<Node> {
	if matches!(node, Node::Meta { .. }) {
		return None; // the definition inside, by map_children, keeps the Meta
	}
	let (head, body, rebuild) = definition_parts(node)?;
	let own = own_names(&head, &body);
	let seen = maps.iter().filter(|name| !own.contains(*name)).cloned().chain(map_variables(&body)).collect();
	Some(rebuild(head, rewrite(body, &seen)))
}

/// The parameters of a definition `f(xs) := body` and the names its body assigns
fn own_names(head: &Node, body: &Node) -> HashSet<String> {
	let mut names: HashSet<String> = head.children().iter().skip(1).filter_map(parameter_name).collect();
	body.visit(&mut |part| if let Node::Key(target, Op::Assign, _) = part {
		names.extend(assigned_name(target).map(str::to_string));
	});
	names
}

/// `m` of `m = …` and of `m: map = …`
fn assigned_name(target: &Node) -> Option<&str> {
	match target.drop_meta() {
		Node::Key(name, Op::Colon, declared) if is_map_type(declared) => name.symbol_name(),
		name => name.symbol_name(),
	}
}

/// `{}` or `{1: 5, 2: 6}`: entries only, at least one keyed by a number
fn is_number_keyed(value: &Node) -> bool {
	let Node::List(items, Bracket::Curly, _) = value.drop_meta() else { return false };
	let entry_key = |item: &Node| match item.drop_meta() {
		Node::Key(key, Op::Colon, _) => Some(key.drop_meta().clone()),
		_ => None,
	};
	let keys: Option<Vec<Node>> = items.iter().map(entry_key).collect();
	keys.is_some_and(|keys| keys.is_empty() || keys.iter().any(|key| matches!(key, Node::Number(_))))
}

/// `{1: 5}` → `{"1": 5}`
fn text_keyed(literal: Node) -> Node {
	match literal {
		Node::List(items, Bracket::Curly, separator) => {
			let entry = |item: Node| match item.drop_meta().clone() {
				Node::Key(key, Op::Colon, value) => Node::Key(Box::new(key_text(*key)), Op::Colon, value),
				_ => item,
			};
			Node::List(items.into_iter().map(entry).collect(), Bracket::Curly, separator)
		}
		Node::Meta { data, node } => Node::Meta { data, node: Box::new(text_keyed(*node)) },
		other => other,
	}
}

/// `map`, `dict`, `map<int, int>` (one symbol, `map of int, int`)
fn is_map_type(declared: &Node) -> bool {
	declared.name().split_whitespace().next().is_some_and(|word| MAP_TYPE_WORDS.contains(&word))
}

fn is_map(node: &Node, maps: &HashSet<String>) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if maps.contains(name))
}

fn keyed(node: Node, maps: &HashSet<String>) -> Node {
	if let Some(scoped) = in_function_scope(&node, maps, keyed) {
		return scoped;
	}
	match node {
		Node::Key(target, Op::Hash, index) if is_map(&target, maps) => {
			let key = match (written_index(&index), crate::warp_parser::subscript_key(&index)) {
				(Some(number), _) => Node::Text(number.to_string()),
				(None, Some(key)) => key_text(keyed(key.clone(), maps)),
				(None, None) => return Node::Key(target, Op::Hash, Box::new(keyed(*index, maps))),
			};
			Node::Key(target, Op::Hash, Box::new(Node::Key(Box::new(key), Op::Add, Box::new(crate::node::int(1)))))
		}
		// `d.remove(k)`, lowered by the analyzer after lower_key_words
		Node::Key(target, Op::Dot, call) if is_map(&target, maps) => match call.drop_meta() {
			Node::List(items, bracket, separator) if matches!(items.as_slice(), [method, _] if method.name() == crate::analyzer::REMOVE_METHOD) => {
				let removed = vec![items[0].clone(), key_text(keyed(items[1].clone(), maps))];
				Node::Key(target, Op::Dot, Box::new(Node::List(removed, bracket.clone(), separator.clone())))
			}
			_ => Node::Key(target, Op::Dot, Box::new(keyed(*call, maps))),
		},
		other => other.map_children(|child| keyed(child, maps)),
	}
}

fn found_by_key(node: Node, maps: &HashSet<String>) -> Node {
	if let Some(scoped) = in_function_scope(&node, maps, found_by_key) {
		return scoped;
	}
	match node {
		Node::Key(target, Op::Assign, value) if assigned_name(&target).is_some_and(|name| maps.contains(name)) => {
			Node::Key(target, Op::Assign, Box::new(text_keyed(found_by_key(*value, maps))))
		}
		Node::List(mut items, Bracket::Round, separator) if is_key_word_call(&items, maps) => {
			items[2] = key_text(found_by_key(items[2].clone(), maps));
			Node::List(items, Bracket::Round, separator)
		}
		other => other.map_children(|child| found_by_key(child, maps)),
	}
}

fn is_key_word_call(items: &[Node], maps: &HashSet<String>) -> bool {
	matches!(items, [word, map, _, ..] if matches!(word.drop_meta(), Node::Symbol(word) if KEY_WORDS.contains(&word.as_str())) && is_map(map, maps))
}

/// The key a value names: a written text or character as it is, anything else its text
fn key_text(key: Node) -> Node {
	match key.drop_meta() {
		Node::Text(_) | Node::Char(_) => key,
		Node::Number(number) => Node::Text(number.to_string()),
		_ => call(KEY_TEXT, vec![key]),
	}
}

fn written_index(index: &Node) -> Option<crate::extensions::numbers::Number> {
	match index {
		Node::Meta { data, node } => match data.as_ref() {
			Node::Data(dada) => dada.downcast_ref::<WrittenIndex>().map(|WrittenIndex(number)| *number).or_else(|| written_index(node)),
			_ => written_index(node),
		},
		_ => None,
	}
}
