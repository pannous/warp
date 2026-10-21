//! `do block` and `do name` run the block on the spot, like `{…}!` and `f!`: the word is dropped. (`add x to list`, the
//! method call `list.add(x)`, is a rewrite rule of lib/english.warp; `do` is a keyword no rule pattern can start with.)

use super::nodes::{children_rewritten, word_headed};
use crate::node::{Bracket, Node};
use crate::words::DO_WORD;

pub fn lower(node: Node) -> Node {
	if !node.mentions(DO_WORD) || crate::analyzer::function_definitions(&node).user_functions.contains_key(DO_WORD) {
		return node;
	}
	Phrases.expand(node)
}

struct Phrases;

impl Phrases {
	fn expand(&self, node: Node) -> Node {
		match node {
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.expand(item)).collect();
				self.phrase(&items).unwrap_or(Node::List(items, bracket, separator))
			}
			other => children_rewritten(other, |child| self.expand(child)),
		}
	}

	fn phrase(&self, items: &[Node]) -> Option<Node> {
		let (word, rest) = word_headed(items)?;
		if word != DO_WORD {
			return None;
		}
		match rest {
			[argument] => evaluated_block(argument),
			// `do pixel.add(4)` of `do add 4 to pixel`: the phrase after `do`
			[_, _, ..] => self.phrase(rest),
			[] => None,
		}
	}
}

/// The block or name of `do block`
fn evaluated_block(argument: &Node) -> Option<Node> {
	matches!(argument.drop_meta(), Node::Symbol(_) | Node::List(_, Bracket::Curly, _)).then(|| argument.clone())
}
