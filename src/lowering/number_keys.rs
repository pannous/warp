//! A number subscript on a variable that starts as the empty map `{}` keys it, like a text subscript (user decision
//! P34): `d={}; d[1]="a"` is {1:"a"} and `d[1]` reads that entry. On any other value `d[1]` stays a position.
//! A number known only at run time keys it too, by its digits: `d[k]` is `d[string(k)]`, and so is a membership test
//! `k in d`, `d has k`, `d.remove(k)` and `d.get(k)` (lower_key_words, after library_words names them), so a map keyed
//! by ids stays a hash table.

use crate::library_words::{COLLECTION_CONTAINS, COLLECTION_POSITION, MAP_GET_OR, MAP_WITHOUT};
use crate::node::{Bracket, Node, Separator};
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
	let maps = empty_map_variables(&node);
	if maps.is_empty() {
		return node;
	}
	keyed(node, &maps)
}

/// `collection_contains(d, k)`, `map_without(d, k)` … (KEY_WORDS) of a map `d` that starts as `{}` find the key `string(k)`
pub fn lower_key_words(node: Node) -> Node {
	let maps = empty_map_variables(&node);
	if maps.is_empty() {
		return node;
	}
	found_by_key(node, &maps)
}

/// The variables assigned the empty map `{}`, also when declared a map (`m: map<int, int> = {}`, card typed-map)
fn empty_map_variables(node: &Node) -> HashSet<String> {
	let mut maps = HashSet::new();
	node.visit(&mut |part| if let Node::Key(target, Op::Assign, value) = part {
		let Node::List(items, Bracket::Curly, _) = value.drop_meta() else { return };
		let name = match target.drop_meta() {
			Node::Key(name, Op::Colon, declared) if is_map_type(declared) => name.drop_meta(),
			name => name,
		};
		if let (Node::Symbol(name), true) = (name, items.is_empty()) {
			maps.insert(name.clone());
		}
	});
	maps
}

/// `map`, `dict`, `map<int, int>` (one symbol, `map of int, int`)
fn is_map_type(declared: &Node) -> bool {
	declared.name().split_whitespace().next().is_some_and(|word| MAP_TYPE_WORDS.contains(&word))
}

fn is_map(node: &Node, maps: &HashSet<String>) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if maps.contains(name))
}

fn keyed(node: Node, maps: &HashSet<String>) -> Node {
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
	match node {
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
		_ => Node::List(vec![Node::Symbol(KEY_TEXT.to_string()), key], Bracket::Round, Separator::None),
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
