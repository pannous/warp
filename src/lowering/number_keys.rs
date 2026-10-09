//! A number subscript on a variable that starts as the empty map `{}` keys it, like a text subscript (user decision
//! P34): `d={}; d[1]="a"` is {1:"a"} and `d[1]` reads that entry. On any other value `d[1]` stays a position.
//! A number known only at run time keys it too, by its digits: `d[k]` is `d[string(k)]`, and so is a membership test
//! `k in d`, `d has k` (lower_membership, after library_words names them), so a map keyed by ids stays a hash table.

use crate::library_words::{COLLECTION_CONTAINS, COLLECTION_POSITION};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::WrittenIndex;
use std::collections::HashSet;

/// The word that makes a key of a number known only at run time
const KEY_TEXT: &str = "string";

pub fn lower(node: Node) -> Node {
	let maps = empty_map_variables(&node);
	if maps.is_empty() {
		return node;
	}
	keyed(node, &maps)
}

/// `collection_contains(d, k)` and `collection_position(d, k)` of a map `d` that starts as `{}` test the key `string(k)`
pub fn lower_membership(node: Node) -> Node {
	let maps = empty_map_variables(&node);
	if maps.is_empty() {
		return node;
	}
	tested_by_key(node, &maps)
}

/// The variables assigned the empty map `{}`
fn empty_map_variables(node: &Node) -> HashSet<String> {
	let mut maps = HashSet::new();
	node.visit(&mut |part| if let Node::Key(target, Op::Assign, value) = part {
		if let (Node::Symbol(name), Node::List(items, Bracket::Curly, _)) = (target.drop_meta(), value.drop_meta()) {
			if items.is_empty() {
				maps.insert(name.clone());
			}
		}
	});
	maps
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
		other => other.map_children(|child| keyed(child, maps)),
	}
}

fn tested_by_key(node: Node, maps: &HashSet<String>) -> Node {
	match node {
		Node::List(mut items, Bracket::Round, separator) if is_membership(&items, maps) => {
			let key = items.pop().expect("a membership test has its key");
			items.push(key_text(tested_by_key(key, maps)));
			Node::List(items, Bracket::Round, separator)
		}
		other => other.map_children(|child| tested_by_key(child, maps)),
	}
}

fn is_membership(items: &[Node], maps: &HashSet<String>) -> bool {
	matches!(items, [word, map, _] if matches!(word.drop_meta(), Node::Symbol(word) if word == COLLECTION_CONTAINS || word == COLLECTION_POSITION) && is_map(map, maps))
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
