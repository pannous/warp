//! The error classes (decision error-classes, notes/error_classes.md): lib/prelude.warp's `class Error`, the classes
//! of the runtime errors, and the classes a program declares extending one of them (`class ParseError extends Error`)
use crate::node::{call, Bracket, Node};
use crate::operators::Op;
use crate::wasm_emitter::text::text_builtins::{RAISE_AS, TEXT_FORM};
use std::collections::{HashMap, HashSet};

/// The class every error is of: `except Error` catches all
pub(crate) const ROOT_ERROR_CLASS: &str = "Error";
/// The classes runtime errors are of (runtime_errors.rs runtime_error_class), with their parents as Python's
const RUNTIME_ERROR_CLASSES: [(&str, &str); 9] = [
	("LookupError", ROOT_ERROR_CLASS),
	("IndexError", "LookupError"),
	("KeyError", "LookupError"),
	("ArithmeticError", ROOT_ERROR_CLASS),
	("ZeroDivisionError", "ArithmeticError"),
	("OverflowError", "ArithmeticError"),
	("ValueError", ROOT_ERROR_CLASS),
	("TypeError", ROOT_ERROR_CLASS),
	("MemoryError", ROOT_ERROR_CLASS),
];

/// `class IndexError extends LookupError {}`: the declaration of a runtime error's class, which a program's class may
/// extend (`class MyIndexError extends IndexError`)
pub(crate) fn runtime_class_declaration(class: &str) -> Option<Node> {
	let (_, parent) = RUNTIME_ERROR_CLASSES.iter().find(|(known, _)| *known == class)?;
	let name = Node::Symbol(class.to_string())
		.with_attribute(crate::warp_parser::EXTENDS_KEYWORD, Node::Symbol(parent.to_string()));
	Some(Node::Type {
		name: Box::new(name),
		body: Box::new(Node::List(vec![], Bracket::Curly, crate::node::Separator::None)),
	})
}

/// The field of an Error holding its message
const MESSAGE_FIELD: &str = "message";

/// Each class's parent, and the names of all the program's types
#[derive(Default)]
pub(crate) struct ErrorClasses {
	parents: HashMap<String, String>,
	types: HashSet<String>,
}

impl ErrorClasses {
	pub(crate) fn of(program: &Node) -> Self {
		let mut parents: HashMap<String, String> =
			RUNTIME_ERROR_CLASSES.iter().map(|(class, parent)| (class.to_string(), parent.to_string())).collect();
		let mut types = HashSet::new();
		program.visit(&mut |part| {
			if let Node::Type { name, .. } = part {
				types.insert(name.drop_meta().name());
				let marks = [crate::warp_parser::EXTENDS_KEYWORD, crate::class_methods::inheritance::INHERITED_FROM];
				if let Some(parent) = marks.iter().find_map(|mark| name.attribute(mark)) {
					parents.insert(name.drop_meta().name(), parent.drop_meta().name());
				}
			}
		});
		ErrorClasses { parents, types }
	}

	/// The class and its ancestors, nearest first
	fn lineage<'a>(&'a self, class: &'a str) -> impl Iterator<Item = &'a str> + 'a {
		let mut seen = vec![];
		std::iter::successors(Some(class), move |class| {
			seen.push(class.to_string());
			self.parents.get(*class).map(String::as_str).filter(|parent| !seen.iter().any(|known| known == parent))
		})
	}

	pub(crate) fn is_error_class(&self, class: &str) -> bool {
		self.lineage(class).any(|ancestor| ancestor == ROOT_ERROR_CLASS)
	}

	/// The class and every class extending it, which `except class` catches
	pub(crate) fn caught_by(&self, class: &str) -> Vec<String> {
		let mut caught: Vec<String> = self
			.parents
			.keys()
			.filter(|known| self.lineage(known).any(|ancestor| ancestor == class))
			.cloned()
			.collect();
		caught.sort();
		[vec![class.to_string()], caught.into_iter().filter(|known| known != class).collect()].concat()
	}

	/// `raise ParseError("bad")`, `raise ParseError{message: "bad"}`, `raise ParseError`: `raise_as(message, "ParseError")`,
	/// an error of the class, its message the instance's (else the class name)
	pub(crate) fn raised_instance(&self, raised: &Node, defined: impl Fn(&str) -> bool) -> Option<Node> {
		let (class, message) = match raised.drop_meta() {
			Node::Symbol(class) => (class.clone(), None),
			Node::List(items, Bracket::Round, _) => (items.first()?.symbol_name()?.to_string(), items.get(1).cloned()),
			Node::Key(class, Op::None | Op::Colon, body) => {
				(class.symbol_name()?.to_string(), entry(body, MESSAGE_FIELD))
			}
			_ => return None,
		};
		if !self.is_error_class(&class) {
			// `raise FooError("x")` of no type nor function: the class to declare
			let undeclared = matches!(raised.drop_meta(), Node::List(..))
				&& class.starts_with(char::is_uppercase)
				&& !self.types.contains(&class)
				&& !defined(&class);
			return undeclared.then(|| {
				crate::node::error(&format!(
					"raise {class}(…): {class} is no error class: declare it with `class {class}: Error`"
				))
			});
		}
		let message = match message.map(|message| message.drop_meta().clone()) {
			Some(Node::Text(message)) => Node::Text(message),
			Some(message) => call(TEXT_FORM, vec![message]),
			None => Node::Text(class.clone()),
		};
		Some(call(RAISE_AS, vec![message, Node::Text(class)]))
	}
}

/// The value of the entry `name: value` in an instance's body
fn entry(body: &Node, name: &str) -> Option<Node> {
	let items = match body.drop_meta() {
		Node::List(items, _, _) => items.as_slice(),
		single => std::slice::from_ref(single),
	};
	items.iter().find_map(|item| match item.drop_meta() {
		Node::Key(key, Op::Colon | Op::Assign, value) if key.is_symbol(name) => Some(value.as_ref().clone()),
		_ => None,
	})
}
