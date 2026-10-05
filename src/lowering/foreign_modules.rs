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
pub const FOREIGN_RUNTIMES: [&str; 3] = ["python", "js", COMPONENT_RUNTIME];
/// `use wasm "lib.wasm" as lib`: a WebAssembly component (src/components.rs); its module is the file, found next to the
/// program, and named after the file by default
pub const COMPONENT_RUNTIME: &str = "wasm";

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
		other if runtime == COMPONENT_RUNTIME => {
			let file = name(other)?;
			let stem = std::path::Path::new(&file).file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_else(|| file.clone());
			(file, stem)
		}
		other => {
			let module = name(other)?;
			(module.clone(), module.rsplit('.').next().unwrap_or(&module).to_string())
		}
	};
	let module = if runtime == COMPONENT_RUNTIME { crate::modules::beside_program(&module) } else { module };
	Some((alias, (runtime.clone(), module)))
}

/// The modules `use` names, and the variables holding a value of another runtime (`d = datetime.date(2020, 1, 2)`):
/// a member of either is reached through foreign_call
struct Foreign<'a> {
	modules: &'a HashMap<String, (String, String)>,
	values: HashMap<String, String>,
}

/// `foreign_call(runtime, …)`: its runtime, when its value can stay in that runtime (a component's results are plain
/// values: a record is an object, `s.words` its field)
fn foreign_runtime(node: &Node) -> Option<String> {
	let Node::List(items, Bracket::Round, _) = node.drop_meta() else { return None };
	match items.as_slice() {
		// `(a * 2)`: the group of one
		[grouped] => foreign_runtime(grouped),
		[call, runtime, ..] if matches!(call.drop_meta(), Node::Symbol(name) if name == crate::host::FOREIGN_CALL) => match runtime.drop_meta() {
			Node::Text(runtime) if runtime == COMPONENT_RUNTIME => None,
			Node::Text(runtime) => Some(runtime.clone()),
			_ => None,
		},
		_ => None,
	}
}

/// The module of operators every runtime offers (Python's operator plus len and list, the loops' own in JS)
const OPERATOR_MODULE: &str = "operator";
const COUNTING_WORDS: [&str; 4] = ["count", "len", "length", "size"];
const FOR_WORD: &str = "for";

/// The operator function an infix operator forwards to: `a * 2` of a handle is `operator.mul(a, 2)`
fn operator_member(op: &Op) -> Option<&'static str> {
	Some(match op {
		Op::Add => "add",
		Op::Sub => "sub",
		Op::Mul => "mul",
		Op::Div => "truediv",
		Op::Mod => "mod",
		Op::Pow => "pow",
		Op::Lt => "lt",
		Op::Gt => "gt",
		Op::Le => "le",
		Op::Ge => "ge",
		Op::Eq => "eq",
		Op::Ne => "ne",
		_ => return None,
	})
}

fn foreign_call(runtime: &str, module: Node, member: &str, is_call: bool, arguments: Node) -> Node {
	let text = |text: &str| Node::Text(text.to_string());
	Node::List(vec![Node::Symbol(crate::host::FOREIGN_CALL.to_string()), text(runtime), module, text(member), Node::int(i64::from(is_call)), arguments], Bracket::Round, Separator::None)
}

/// `operator.member(arguments…)` in `runtime`
fn operator_call(runtime: &str, member: &str, arguments: Vec<Node>) -> Node {
	foreign_call(runtime, Node::Text(OPERATOR_MODULE.to_string()), member, true, Node::List(arguments, Bracket::Square, Separator::Space))
}

impl Foreign<'_> {
	/// The runtime of a value of another runtime: a variable holding one, or a foreign call
	fn foreign_value(&self, node: &Node) -> Option<String> {
		match node.drop_meta() {
			Node::Symbol(name) => self.values.get(name).cloned(),
			other => foreign_runtime(other),
		}
	}

	/// Operators, indexing, counting and iteration of a foreign value forward to its runtime
	fn forwarded(&mut self, node: &Node) -> Option<Node> {
		match node.drop_meta() {
			// `-a`, `#a`
			Node::Key(empty, op @ (Op::Neg | Op::Sub | Op::Hash), operand) if matches!(empty.drop_meta(), Node::Empty) => {
				let operand = self.rewrite(operand.as_ref().clone());
				let runtime = self.foreign_value(&operand)?;
				Some(operator_call(&runtime, if *op == Op::Hash { "len" } else { "neg" }, vec![operand]))
			}
			// `a#2`, 1-based: the item at 0-based 1
			Node::Key(indexed, Op::Hash, index) => {
				let indexed = self.rewrite(indexed.as_ref().clone());
				let runtime = self.foreign_value(&indexed)?;
				let zero_based = Node::Key(Box::new(self.rewrite(index.as_ref().clone())), Op::Sub, Box::new(Node::int(1)));
				Some(operator_call(&runtime, "getitem", vec![indexed, zero_based]))
			}
			Node::Key(left, op, right) if operator_member(op).is_some() => {
				let (left, right) = (self.rewrite(left.as_ref().clone()), self.rewrite(right.as_ref().clone()));
				let runtime = self.foreign_value(&left).or_else(|| self.foreign_value(&right))?;
				Some(operator_call(&runtime, operator_member(op).expect("guarded"), vec![left, right]))
			}
			// `count a`, `len(a)`
			Node::List(items, _, _) if matches!(items.as_slice(), [word, _] if matches!(word.drop_meta(), Node::Symbol(word) if COUNTING_WORDS.contains(&word.as_str()))) => {
				let counted = self.rewrite(items[1].clone());
				let runtime = self.foreign_value(&counted)?;
				Some(operator_call(&runtime, "len", vec![counted]))
			}
			// `for x in a { … }`: over the items of a, as a list
			Node::List(items, bracket, separator) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == FOR_WORD) && items.len() >= 4 => {
				let iterated = self.rewrite(items[3].clone());
				let runtime = self.foreign_value(&iterated)?;
				let mut items: Vec<Node> = items.iter().cloned().map(|item| self.rewrite(item)).collect();
				items[3] = operator_call(&runtime, "list", vec![iterated]);
				Some(Node::List(items, bracket.clone(), separator.clone()))
			}
			_ => None,
		}
	}

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
		if let Some(forwarded) = self.forwarded(&node) {
			return forwarded;
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
				return foreign_call(&runtime, module, &member, is_call, arguments);
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
