//! A function value called where it is picked from a list: `fs[1](3)` (parsed as the pair `(fs#2) (3)`) becomes
//! `(picked·1 = fs#2; picked·1(3))`, the call of a function held by a variable, which closures.rs knows.

use super::nodes::{key, Counter};
use crate::analyzer::call_name;
use crate::node::{parens, Bracket, Node, Separator};
use crate::operators::Op;

const PICKED_PREFIX: &str = "picked·";

pub fn lower(node: Node) -> Node {
	Picking { counter: Counter::default() }.rewrite(node)
}

struct Picking {
	counter: Counter,
}

impl Picking {
	fn rewrite(&self, node: Node) -> Node {
		match node {
			Node::List(items, bracket, separator)
				if is_pair(&items, &bracket, &separator) && self.called(&items[0], &items[1]).is_some() =>
			{
				let called = self.called(&items[0], &items[1]).expect("guarded");
				if bracket == Bracket::Curly {
					Node::List(vec![called], bracket, separator)
				} else {
					called
				}
			}
			other => other.map_children(|child| self.rewrite(child)),
		}
	}

	/// The group of arguments applied to the operand right before it: the indexed element itself, or the last operand
	/// of an expression the parser closed first (`s += fs[i](3)` reads `(s += fs#(i+1)) (3)`)
	fn called(&self, operand: &Node, arguments: &Node) -> Option<Node> {
		let arguments = match arguments.drop_meta() {
			// `osc(A4, 1s) |> rooms#1` applies rooms#1 to the one argument osc(A4, 1s), a call, not a group of three
			Node::List(items, bracket, separator)
				if items.len() > 1 && call_name(items, bracket, separator).is_some() =>
			{
				std::slice::from_ref(arguments)
			}
			Node::List(arguments, Bracket::Round, _) => arguments.as_slice(),
			_ => return None,
		};
		match operand.drop_meta() {
			Node::Key(_, Op::Hash, index) if !index.is_empty_node() => {
				Some(self.picked_call(operand.clone(), arguments))
			}
			Node::Key(left, op, right) => self
				.called(right, &parens(arguments.to_vec()))
				.map(|right| key(self.rewrite(left.as_ref().clone()), *op, right)),
			_ => None,
		}
	}

	fn picked_call(&self, picked: Node, arguments: &[Node]) -> Node {
		let name = format!("{PICKED_PREFIX}{}", self.counter.next_number());
		let assignment = key(Node::Symbol(name.clone()), Op::Assign, self.rewrite(picked));
		let arguments = arguments.iter().map(|argument| self.rewrite(argument.clone()));
		let call = parens(std::iter::once(Node::Symbol(name)).chain(arguments).collect());
		Node::List(vec![assignment, call], Bracket::Round, Separator::Semicolon)
	}
}

/// Two items juxtaposed, standing alone or as the one statement of a block
fn is_pair(items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
	items.len() == 2
		&& matches!(bracket, Bracket::None | Bracket::Round | Bracket::Curly)
		&& matches!(separator, Separator::None | Separator::Space)
}
