//! Listener statements read one way for every signal pass (notes/cleanup.md, the signal family): `on stop the machine
//! {body}`, `on every 5 seconds: body`, `once x == 5 {body}`, `on change p.age do body`, `h = on alarm {body}`. The
//! parser groups such a phrase as any other (`(on change) (s {…})`, `on every (5 seconds: print) n`, `a == (b {body})`,
//! `(alarm = (whenever t) > 30) {…}`); the reader undoes the groups: the keyword, what it listens to, and the body.

use super::nodes::{phrase_words, phrase_words_of};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

pub(crate) struct Listener {
	/// `h` of `h = on alarm {…}`
	pub name: Option<String>,
	/// `on`, `once`, `whenever`, `at` …
	pub keyword: String,
	/// what follows the keyword: `stop the machine`, `x == 5`, `change p.age`
	pub trigger: Node,
	pub body: Node,
}

impl Listener {
	/// The trigger's words in one row: `every 5 seconds` → [every, 5, seconds]
	pub fn words(&self) -> Vec<Node> {
		phrase_words_of(&self.trigger)
	}

	/// The listener as a statement of its own, without its name
	pub fn unnamed(&self) -> Node {
		let keyword = Node::Symbol(self.keyword.clone());
		Node::List(vec![keyword, self.trigger.clone(), self.body.clone()], Bracket::None, Separator::Space)
	}
}

/// The listener a statement is, when it starts with one of `keywords` and is not named
pub(crate) fn read(statement: &Node, keywords: &[&str]) -> Option<Listener> {
	read_named(statement, keywords).filter(|listener| listener.name.is_none())
}

/// The listener a statement is, named (`h = on alarm {…}`) or not
pub(crate) fn read_named(statement: &Node, keywords: &[&str]) -> Option<Listener> {
	let (head, body) = head_and_body(statement)?;
	let (name, head) = match head.drop_meta() {
		Node::Key(name, Op::Assign, value) => (Some(name.symbol_name()?.to_string()), value.as_ref().clone()),
		_ => (None, head),
	};
	let (keyword, trigger) = leading_word(&head)?;
	let keyword = keyword.symbol_name().filter(|word| keywords.contains(word))?.to_string();
	Some(Listener { name, keyword, trigger, body })
}

/// `… {body}`, `…: body`, `… do body`, `a == (b {body})`, `on every (5 seconds: print) n`: the phrase before the body,
/// and the body
fn head_and_body(node: &Node) -> Option<(Node, Node)> {
	match node.drop_meta() {
		Node::Key(head, Op::Colon | Op::Do, body) => Some((head.as_ref().clone(), body.as_ref().clone())),
		// `on alarm: x = 1` arrives as `(on alarm: x) = 1`
		Node::Key(head, op, value) if matches!(head.drop_meta(), Node::Key(_, Op::Colon, _)) => {
			let (event, target) = head_and_body(head)?;
			Some((event, Node::Key(Box::new(target), *op, value.clone())))
		}
		Node::Key(left, op, right) => {
			let (right, body) = head_and_body(right)?;
			Some((Node::Key(left.clone(), *op, Box::new(right)), body))
		}
		// `whenever big() {…}` arrives as `whenever ((big ø) {…})`
		Node::List(items, Bracket::Round, _) if items.len() == 2 && items[1].is_block() => {
			Some((items[0].clone(), items[1].clone()))
		}
		// a block of one listener, `{on click {…}}` as a loop's or function's body, is its words in braces
		Node::List(items, Bracket::None | Bracket::Curly, Separator::Space) => {
			let items = phrase_words(items);
			let (last, words) = items.split_last()?;
			if last.is_block() {
				return Some((row(words.to_vec()), last.clone()));
			}
			let colon = items.iter().position(|item| matches!(item.drop_meta(), Node::Key(_, Op::Colon, _)));
			match colon {
				// the words after the colon are the body's
				Some(colon) if colon + 1 < items.len() => {
					let (end, body) = head_and_body(&items[colon])?;
					let body = row([vec![body], items[colon + 1..].to_vec()].concat());
					Some((row([items[..colon].to_vec(), vec![end]].concat()), body))
				}
				_ => {
					let (end, body) = head_and_body(last)?;
					Some((row([words.to_vec(), vec![end]].concat()), body))
				}
			}
		}
		_ => None,
	}
}

/// `whenever t > 30` read as `(whenever t) > 30` too: the word the phrase starts with, and the rest
pub(crate) fn leading_word(node: &Node) -> Option<(Node, Node)> {
	match node.drop_meta() {
		Node::Key(left, op, right) => {
			let (word, left) = leading_word(left)?;
			Some((word, Node::Key(Box::new(left), *op, right.clone())))
		}
		Node::List(items, Bracket::None, Separator::Space) if items.len() >= 2 => {
			let items = phrase_words(items);
			Some((items[0].clone(), row(items[1..].to_vec())))
		}
		_ => None,
	}
}

/// One node of words: the word itself, else the words in a row
pub(crate) fn row(mut words: Vec<Node>) -> Node {
	match words.len() {
		1 => words.remove(0),
		_ => Node::List(words, Bracket::None, Separator::Space),
	}
}
