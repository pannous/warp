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
pub const FOREIGN_RUNTIMES: [&str; 3] = ["python", JS_RUNTIME, COMPONENT_RUNTIME];
/// JavaScript: its browser globals are typed through WebIDL (src/web_idl.rs)
const JS_RUNTIME: &str = "js";
/// `use wasm "lib.wasm" as lib`: a WebAssembly component (src/components.rs); its module is the file, found next to the
/// program, and named after the file by default
pub const COMPONENT_RUNTIME: &str = "wasm";
const COMPONENT_EXTENSION: &str = ".wasm";
/// The version after `\0asm` in a core module's header; a component's names another layer
const CORE_MODULE_VERSION: [u8; 4] = [1, 0, 0, 0];

pub fn lower(program: Node) -> Node {
	let program = component_uses(program);
	let mut modules = HashMap::new();
	program.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			modules.extend(foreign_use(items));
		}
	});
	if modules.is_empty() {
		return program;
	}
	Foreign { modules: &modules, values: HashMap::new(), component_values: Default::default() }.rewrite(program)
}

/// `use rust_demo.wasm`: the component's long form `use wasm "rust_demo.wasm"` (card g-_Xm4); a core module file stays
/// the import of its functions (modules.rs)
fn component_uses(node: Node) -> Node {
	let path = match &node {
		Node::List(items, _, _) => component_path(items),
		_ => None,
	};
	match (path, node) {
		(Some(path), Node::List(_, bracket, separator)) => Node::List(vec![Node::Symbol(USE_WORD.to_string()), Node::Symbol(COMPONENT_RUNTIME.to_string()), Node::Text(path)], bracket, separator),
		(_, node) => node.map_children(component_uses),
	}
}

fn component_path(items: &[Node]) -> Option<String> {
	let [word, file] = items else { return None };
	if !matches!(word.drop_meta(), Node::Symbol(word) if crate::modules::USE_KEYWORDS.contains(&word.as_str())) {
		return None;
	}
	let path = crate::modules::path_of(file).filter(|path| path.ends_with(COMPONENT_EXTENSION))?;
	(!is_core_module(&crate::modules::beside_program(&path))).then_some(path)
}

/// A file that is no component, else none or unreadable here (the page finds its components by name, components.js)
fn is_core_module(path: &str) -> bool {
	std::fs::read(path).is_ok_and(|bytes| bytes.get(4..8) == Some(CORE_MODULE_VERSION.as_slice()))
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
	/// variables assigned a component's result: plain values, or handles of its resources whose methods it runs
	component_values: std::collections::HashSet<String>,
}

/// `foreign_call(runtime, …)`: its runtime, when its value can stay in that runtime (a component's results are plain
/// values: a record is an object, `s.words` its field, `n * 2` warp's arithmetic)
fn foreign_runtime(node: &Node) -> Option<String> {
	called_runtime(node).filter(|runtime| runtime != COMPONENT_RUNTIME)
}

/// `foreign_call(runtime, …)`, also in a group of one: its runtime
fn called_runtime(node: &Node) -> Option<String> {
	let Node::List(items, Bracket::Round, _) = node.drop_meta() else { return None };
	match items.as_slice() {
		// `(a * 2)`: the group of one
		[grouped] => called_runtime(grouped),
		[call, runtime, ..] if matches!(call.drop_meta(), Node::Symbol(name) if name == crate::host::FOREIGN_CALL) => match runtime.drop_meta() {
			Node::Text(runtime) => Some(runtime.clone()),
			_ => None,
		},
		_ => None,
	}
}

/// The items of a method call `name(arguments…)` after a dot
fn called_method(member: &Node) -> Option<&Vec<Node>> {
	match member.drop_meta() {
		Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => Some(items),
		_ => None,
	}
}

/// A word warp's own values answer as a method: `xs.reverse()`, `t.upper()`; on a component's plain result it stays
/// warp's, any other method is the component's (a resource's: `counter.increment(2)`)
fn is_warp_method(name: &str) -> bool {
	crate::library_words::is_library_word(name) || crate::analyzer::is_list_mutating_method(name)
		|| (1..=3).any(|arity| crate::wasm_emitter::text_builtins::text_builtin_kind(name, arity).is_some())
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

/// `crypto.randomUuid()` of `use js crypto`: what WebIDL declares for that global instead (a module, not a held value)
fn undeclared_in_web_idl(runtime: &str, module: &Node, member: &str, is_call: bool, arguments: &Node) -> Option<String> {
	let Node::Text(global) = module else { return None };
	if runtime != JS_RUNTIME {
		return None;
	}
	let count = match arguments {
		Node::List(items, _, _) => items.len(),
		_ => 0,
	};
	crate::web_idl::check(global, member, is_call.then_some(count)).err()
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

	/// A variable assigned a component's result, or a component's call itself
	fn holds_component_value(&self, receiver: &Node) -> bool {
		match receiver.drop_meta() {
			Node::Symbol(name) => self.component_values.contains(name),
			other => called_runtime(other).is_some_and(|runtime| runtime == COMPONENT_RUNTIME),
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
				if let Some(problem) = undeclared_in_web_idl(&runtime, &module, &member, is_call, &arguments) {
					return crate::diagnostic::Diagnostic::at(&node, problem).into_error();
				}
				return foreign_call(&runtime, module, &member, is_call, arguments);
			}
			// `counter.increment(2)` of a handle a component gave: the method of its resource
			if let Some(items) = called_method(member).filter(|items| self.holds_component_value(&receiver) && !is_warp_method(&items[0].name())) {
				let arguments = items[1..].iter().cloned().map(|argument| self.rewrite(argument)).collect();
				return foreign_call(COMPONENT_RUNTIME, receiver, &items[0].name(), true, Node::List(arguments, Bracket::Square, Separator::Space));
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
					match called_runtime(&value).is_some_and(|runtime| runtime == COMPONENT_RUNTIME) {
						true => self.component_values.insert(name.clone()),
						false => self.component_values.remove(name),
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
