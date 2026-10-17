//! Word phrases that are sugar for one construct:
//! `do block` and `do name` run the block on the spot, like `{…}!` and `f!`: the word is dropped;
//! `add x to list` is the method call `list.add(x)`.

use super::nodes::{children_rewritten, word_headed};
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::ADD_WORD;
use crate::words::DO_WORD;

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&[DO_WORD, ADD_WORD]) {
		return node;
	}
	let context = crate::analyzer::function_definitions(&node);
	let phrases = Phrases {
		do_is_free: !context.user_functions.contains_key(DO_WORD),
		add_is_free: !context.user_functions.contains_key(ADD_WORD),
	};
	phrases.expand(node)
}

struct Phrases {
	do_is_free: bool,
	add_is_free: bool,
}

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
		// `do add 4 to pixel`: the phrase after `do` is lowered first
		if word == DO_WORD && self.do_is_free && rest.len() > 1 {
			return self.phrase(rest);
		}
		let [argument] = rest else { return None };
		match word {
			DO_WORD if self.do_is_free => evaluated_block(argument),
			ADD_WORD if self.add_is_free => appended_to(argument),
			_ => None,
		}
	}
}

/// The block or name of `do block`
fn evaluated_block(argument: &Node) -> Option<Node> {
	matches!(argument.drop_meta(), Node::Symbol(_) | Node::List(_, Bracket::Curly, _)).then(|| argument.clone())
}

/// `add x to list` parses as `add (x to list)`: the call `list.add(x)`
fn appended_to(argument: &Node) -> Option<Node> {
	let Node::Key(element, Op::To, list) = argument.drop_meta() else {
		return None;
	};
	if !matches!(list.drop_meta(), Node::Symbol(_)) {
		return None;
	}
	let call = Node::List(vec![symbol(ADD_WORD), element.as_ref().clone()], Bracket::Round, Separator::Space);
	Some(Node::Key(list.clone(), Op::Dot, Box::new(call)))
}
