//! Word phrases that are sugar for one construct:
//! `do block` and `do name` run the block on the spot, like `{…}!` and `f!`: the word is dropped;
//! `add x to list` is the method call `list.add(x)`.

use crate::analyzer::extract_user_functions;
use crate::context::Context;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const DO_WORD: &str = "do";
const ADD_WORD: &str = "add";

pub fn lower(node: Node) -> Node {
	let mut context = Context::new();
	extract_user_functions(&mut context, &node);
	let phrases = Phrases { do_is_free: !context.user_functions.contains_key(DO_WORD), add_is_free: !context.user_functions.contains_key(ADD_WORD) };
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
			Node::Key(left, op, right) => Node::Key(Box::new(self.expand(*left)), op, Box::new(self.expand(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.expand(*node)), data },
			other => other,
		}
	}

	fn phrase(&self, items: &[Node]) -> Option<Node> {
		let (word, rest) = items.split_first()?;
		let Node::Symbol(word) = word.drop_meta() else { return None };
		// `do add 4 to pixel`: the phrase after `do` is lowered first
		if word == DO_WORD && self.do_is_free && rest.len() > 1 {
			return self.phrase(rest);
		}
		let [argument] = rest else { return None };
		match word.as_str() {
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
	let Node::Key(element, Op::To, list) = argument.drop_meta() else { return None };
	if !matches!(list.drop_meta(), Node::Symbol(_)) {
		return None;
	}
	let call = Node::List(vec![Node::Symbol(ADD_WORD.to_string()), element.as_ref().clone()], Bracket::Round, Separator::Space);
	Some(Node::Key(list.clone(), Op::Dot, Box::new(call)))
}
