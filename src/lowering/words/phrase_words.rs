//! `do block` and `do name` run the block on the spot, like `{…}!` and `f!`; `do pixel.add(4)` (of `do add 4 to pixel`)
//! runs the call: the word is dropped. (`add x to list`, the method call `list.add(x)`, is a rewrite rule of
//! lib/english.warp; `do` is a keyword no rule pattern can start with.)

use super::nodes::{children_rewritten, word_headed};
use crate::node::{Bracket, Node};
use crate::words::DO_WORD;

pub fn lower(node: Node) -> Node {
	if !node.mentions(DO_WORD) || crate::analyzer::function_definitions(&node).user_functions.contains_key(DO_WORD) {
		return node;
	}
	without_do(node)
}

fn without_do(node: Node) -> Node {
	match node {
		Node::List(items, Bracket::None, separator) => {
			let items: Vec<Node> = items.into_iter().map(without_do).collect();
			match word_headed(&items) {
				Some((DO_WORD, [argument])) => argument.clone(),
				_ => Node::List(items, Bracket::None, separator),
			}
		}
		other => children_rewritten(other, without_do),
	}
}
