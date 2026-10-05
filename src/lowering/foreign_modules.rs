//! `use python "math"` (also `use python math`, `use python "os.path" as path`): the module of another runtime, reached
//! through the host word foreign_call (src/foreign.rs, notes/stdlib_connectors.md). `math.sqrt(2)` is the call
//! `foreign_call("python", "math", "sqrt", [2])`, `math.pi` the read `foreign_call("python", "math", "pi", ø)`.
//! Values cross as JSON; the result is any Node.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashMap;

const USE_WORD: &str = "use";
/// The runtimes a `use <runtime> <module>` names
pub const FOREIGN_RUNTIMES: [&str; 1] = ["python"];

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
	rewrite(program, &modules)
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

fn rewrite(node: Node, modules: &HashMap<String, (String, String)>) -> Node {
	if let Node::List(items, _, _) = node.drop_meta() {
		if foreign_use(items).is_some() {
			return Node::Empty;
		}
	}
	if let Node::Key(receiver, Op::Dot, member) = node.drop_meta() {
		let alias = match receiver.drop_meta() { Node::Symbol(alias) => Some(alias), _ => None };
		if let Some((runtime, module)) = alias.and_then(|alias| modules.get(alias)) {
			let (member, arguments) = match member.drop_meta() {
				Node::Symbol(member) => (member.clone(), Node::Empty),
				Node::List(items, Bracket::Round, _) => match items.split_first() {
					Some((head, arguments)) if matches!(head.drop_meta(), Node::Symbol(_)) => {
						let arguments = arguments.iter().cloned().map(|argument| rewrite(argument, modules)).collect();
						(head.name(), Node::List(arguments, Bracket::Square, Separator::Space))
					}
					_ => return node,
				},
				_ => return node,
			};
			let text = |text: &str| Node::Text(text.to_string());
			return Node::List(vec![Node::Symbol(crate::host::FOREIGN_CALL.to_string()), text(runtime), text(module), text(&member), arguments], Bracket::Round, Separator::None);
		}
	}
	match node {
		Node::Key(left, op, right) => Node::Key(Box::new(rewrite(*left, modules)), op, Box::new(rewrite(*right, modules))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| rewrite(item, modules)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(rewrite(*node, modules)), data },
		other => other,
	}
}
