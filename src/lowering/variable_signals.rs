//! Signal keywords on variables (wiki/signal.md): `once x==5 {…}` runs its body the first time its condition holds
//! after a change of a variable the condition reads, `whenever x>1 {…}` each time. A module runs on one thread, so the
//! listener is its check after every later write of such a variable in the statements that follow it, loop bodies
//! included: a write inside an expression (`while x-->0`) becomes `(x--; check; x+1)`, keeping the expression's value.

use crate::declarations::{handler_parts, word};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;

const ONCE_WORD: &str = "once";
const WHENEVER_WORD: &str = "whenever";
/// `once_fired_0`: whether the first once listener ran
const FIRED_PREFIX: &str = "once_fired_";

#[derive(Clone)]
struct Listener {
	condition: Node,
	body: Node,
	/// the flag of a once listener, None for whenever
	fired: Option<String>,
	watched: HashSet<String>,
}

impl Listener {
	fn check(&self) -> Node {
		match &self.fired {
			None => if_then(self.condition.clone(), block(vec![self.body.clone()])),
			Some(fired) => {
				let not_fired = Node::Key(Box::new(Node::Empty), Op::Not, Box::new(Node::Symbol(fired.clone())));
				let condition = Node::Key(Box::new(not_fired), Op::And, Box::new(self.condition.clone()));
				if_then(condition, block(vec![assign(fired, Node::True), self.body.clone()]))
			}
		}
	}
}

pub fn lower(node: Node) -> Node {
	Signals { count: 0 }.lower(node, &[])
}

struct Signals {
	/// once listeners so far: each gets its own flag
	count: usize,
}

impl Signals {
	fn lower(&mut self, node: Node, listeners: &[Listener]) -> Node {
		match node {
			Node::List(items, bracket, separator) if is_statement_list(&bracket, &separator) => {
				Node::List(self.statements(items, listeners), bracket, separator)
			}
			Node::List(items, bracket, separator) => {
				Node::List(items.into_iter().map(|item| self.lower(item, listeners)).collect(), bracket, separator)
			}
			Node::Key(target, op, value) => {
				let written = written_variable(&target, op).filter(|name| watches(listeners, name));
				let node = Node::Key(target, op, Box::new(self.lower(*value, listeners)));
				match written {
					Some(name) => {
						let value_after = value_after_write(&node);
						let mut parts = vec![node];
						parts.extend(checks(listeners, &name));
						parts.push(value_after);
						Node::List(parts, Bracket::Round, Separator::Semicolon)
					}
					None => match node {
						Node::Key(target, op, value) => Node::Key(Box::new(self.lower(*target, listeners)), op, value),
						_ => unreachable!("built as a key above"),
					},
				}
			}
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.lower(*node, listeners)), data },
			other => other,
		}
	}

	/// A listener applies to the statements after it; a write as a whole statement is followed by the checks
	fn statements(&mut self, items: Vec<Node>, outer: &[Listener]) -> Vec<Node> {
		let mut listeners = outer.to_vec();
		let mut out = Vec::new();
		for item in items {
			if let Some(listener) = self.listener(&item) {
				if let Some(fired) = &listener.fired {
					out.push(assign(fired, Node::False));
				}
				listeners.push(listener);
				continue;
			}
			match statement_write(&item).filter(|name| watches(&listeners, name)) {
				Some(name) => {
					let Node::Key(target, op, value) = item.drop_meta().clone() else { unreachable!("a write is a key") };
					out.push(Node::Key(target, op, Box::new(self.lower(*value, &listeners))));
					out.extend(checks(&listeners, &name));
				}
				None => out.push(self.lower(item, &listeners)),
			}
		}
		out
	}

	/// `once x==5 {body}`, `whenever x>1 : body`
	fn listener(&mut self, statement: &Node) -> Option<Listener> {
		let Node::List(items, _, _) = statement.drop_meta() else { return None };
		let keyword = word(items.first()?);
		if keyword != ONCE_WORD && keyword != WHENEVER_WORD {
			return None;
		}
		let (condition, body) = match &items[1..] {
			[condition, body] if matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) => (condition.clone(), body.clone()),
			[handler] => handler_parts(handler)?,
			_ => return None,
		};
		let mut watched = HashSet::new();
		condition.visit(&mut |part| if let Node::Symbol(name) = part { watched.insert(name.clone()); });
		if watched.is_empty() {
			return None;
		}
		let fired = (keyword == ONCE_WORD).then(|| {
			self.count += 1;
			format!("{FIRED_PREFIX}{}", self.count - 1)
		});
		let body = self.lower(body, &[]);
		Some(Listener { condition, body, fired, watched })
	}
}

fn is_statement_list(bracket: &Bracket, separator: &Separator) -> bool {
	*bracket == Bracket::Curly || matches!(separator, Separator::Semicolon | Separator::Newline)
}

fn watches(listeners: &[Listener], name: &str) -> bool {
	listeners.iter().any(|listener| listener.watched.contains(name))
}

fn checks(listeners: &[Listener], name: &str) -> Vec<Node> {
	listeners.iter().filter(|listener| listener.watched.contains(name)).map(Listener::check).collect()
}

/// The variable `x = …`, `x += …`, `x++`, `x--` writes
fn written_variable(target: &Node, op: Op) -> Option<String> {
	let writes = op == Op::Assign || op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec);
	match target.drop_meta() {
		Node::Symbol(name) if writes => Some(name.clone()),
		_ => None,
	}
}

fn statement_write(statement: &Node) -> Option<String> {
	match statement.drop_meta() {
		Node::Key(target, op, _) => written_variable(target, *op),
		_ => None,
	}
}

/// What the write gave: `x--` the value before it, `x+1`; any other write the new value
fn value_after_write(write: &Node) -> Node {
	let Node::Key(target, op, _) = write else { unreachable!("a write is a key") };
	let undo = match op {
		Op::Dec => Op::Add,
		Op::Inc => Op::Sub,
		_ => return target.as_ref().clone(),
	};
	Node::Key(target.clone(), undo, Box::new(crate::node::int(1)))
}

fn if_then(condition: Node, body: Node) -> Node {
	let head = Node::Key(Box::new(Node::Empty), Op::If, Box::new(condition));
	Node::Key(Box::new(head), Op::Then, Box::new(body))
}

fn block(statements: Vec<Node>) -> Node {
	Node::List(statements, Bracket::Curly, Separator::Semicolon)
}

fn assign(name: &str, value: Node) -> Node {
	Node::Key(Box::new(Node::Symbol(name.to_string())), Op::Assign, Box::new(value))
}
