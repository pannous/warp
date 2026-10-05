//! `use python "math"` (also `use python math`, `use python "os.path" as path`, `use js Math`): the module of another runtime, reached
//! through the host word foreign_call (src/foreign.rs, notes/stdlib_connectors.md). `math.sqrt(2)` is the call
//! `foreign_call("python", "math", "sqrt", 1, [2])`, `math.pi` the read `foreign_call("python", "math", "pi", 0, ø)`
//! (an empty argument list is ø on the way, so whether it is a call is said apart).
//! Values cross as JSON; the result is any Node.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashMap;

const USE_WORD: &str = "use";
/// The runtimes a `use <runtime> <module>` names: Python, and JavaScript (node natively, the page in the browser)
pub const FOREIGN_RUNTIMES: [&str; 2] = ["python", "js"];

pub fn lower(program: Node) -> Node {
	let mut modules = HashMap::new();
	program.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			modules.extend(foreign_use(items));
		}
	});
	if modules.is_empty() {
		return program;
	}
	Foreign { modules: &modules, values: HashMap::new() }.rewrite(program)
}

/// `use python "math"` → (alias math, (python, math)); `use python "os.path" as path` → (path, (python, os.path))
fn foreign_use(items: &[Node]) -> Option<(String, (String, String))> {
	let [word, runtime, module] = items else { return None };
	if !matches!(word.drop_meta(), Node::Symbol(word) if word == USE_WORD) {
		return None;
	}
	let Node::Symbol(runtime) = runtime.drop_meta() else { return None };
	if !FOREIGN_RUNTIMES.contains(&runtime.as_str()) {
		return None;
	}
	let name = |node: &Node| match node.drop_meta() {
		Node::Symbol(name) | Node::Text(name) => Some(name.clone()),
		_ => None,
	};
	let (module, alias) = match module.drop_meta() {
		Node::Key(module, Op::As, alias) => (name(module)?, name(alias)?),
		other => {
			let module = name(other)?;
			(module.clone(), module.rsplit('.').next().unwrap_or(&module).to_string())
		}
	};
	Some((alias, (runtime.clone(), module)))
}

/// The modules `use` names, and the variables holding a value of another runtime (`d = datetime.date(2020, 1, 2)`):
/// a member of either is reached through foreign_call
struct Foreign<'a> {
	modules: &'a HashMap<String, (String, String)>,
	values: HashMap<String, String>,
}

/// `foreign_call(runtime, …)`: its runtime
fn foreign_runtime(node: &Node) -> Option<String> {
	let Node::List(items, Bracket::Round, Separator::None) = node.drop_meta() else { return None };
	match items.as_slice() {
		[call, runtime, ..] if matches!(call.drop_meta(), Node::Symbol(name) if name == crate::host::FOREIGN_CALL) => match runtime.drop_meta() {
			Node::Text(runtime) => Some(runtime.clone()),
			_ => None,
		},
		_ => None,
	}
}

impl Foreign<'_> {
	/// The runtime and the module (its name as text) or the handle (the value itself) a receiver names
	fn receiver(&self, receiver: &Node) -> Option<(String, Node)> {
		if let Node::Symbol(name) = receiver.drop_meta() {
			if let Some((runtime, module)) = self.modules.get(name) {
				return Some((runtime.clone(), Node::Text(module.clone())));
			}
			if let Some(runtime) = self.values.get(name) {
				return Some((runtime.clone(), receiver.clone()));
			}
		}
		foreign_runtime(receiver).map(|runtime| (runtime, receiver.clone()))
	}

	fn rewrite(&mut self, node: Node) -> Node {
		if let Node::List(items, _, _) = node.drop_meta() {
			if foreign_use(items).is_some() {
				return Node::Empty;
			}
		}
		if let Node::Key(receiver, Op::Dot, member) = node.drop_meta() {
			let receiver = self.rewrite(receiver.as_ref().clone());
			if let Some((runtime, module)) = self.receiver(&receiver) {
				let (member, is_call, arguments) = match member.drop_meta() {
					Node::Symbol(member) => (member.clone(), false, Node::Empty),
					Node::List(items, Bracket::Round, _) => match items.split_first() {
						Some((head, arguments)) if matches!(head.drop_meta(), Node::Symbol(_)) => {
							let arguments = arguments.iter().cloned().map(|argument| self.rewrite(argument)).collect();
							(head.name(), true, Node::List(arguments, Bracket::Square, Separator::Space))
						}
						_ => return node,
					},
					_ => return node,
				};
				let text = |text: &str| Node::Text(text.to_string());
				return Node::List(vec![Node::Symbol(crate::host::FOREIGN_CALL.to_string()), text(&runtime), module, text(&member), Node::int(i64::from(is_call)), arguments], Bracket::Round, Separator::None);
			}
		}
		match node {
			// `d = datetime.date(2020, 1, 2)`: d holds a value of that runtime, `d.isoformat()` asks it
			Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
				let value = self.rewrite(*value);
				if let Node::Symbol(name) = target.drop_meta() {
					match foreign_runtime(&value) {
						Some(runtime) => self.values.insert(name.clone(), runtime),
						None => self.values.remove(name),
					};
				}
				Node::Key(target, op, Box::new(value))
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.rewrite(*left)), op, Box::new(self.rewrite(*right))),
			Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| self.rewrite(item)).collect(), bracket, separator),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.rewrite(*node)), data },
			other => other,
		}
	}
}
