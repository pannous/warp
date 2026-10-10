//! The operations a declared type defines for itself, used where an instance of it is known (InstanceTypes):
//! Printable (user decision P31): `text(p:person) := …` gives the text of a person, the one function a type word may
//! name; `x as text`, `str(x)`, `text(x)`, `print x` and an interpolation hole `"\(x)"` call it.
//! Iterable (wiki/trait.md): `iterate(b:bag) := …` gives what a bag holds; `for x in b` walks it and `x in b` searches it.
//! Each definition becomes its witness (`text·person`, `iterate·bag`); every other value keeps its own text and items.

use super::words::{FOR_WORD, IN_WORD};
use super::nodes::{call, key};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::traits::{witness_name, InstanceTypes, Shape};
use std::collections::HashSet;

const TEXT_OPERATION: &str = "text";
const ITERATE_OPERATION: &str = "iterate";
/// The words whose call or cast gives a value's text
const TEXT_WORDS: [&str; 4] = ["text", "string", "str", "String"];
const PRINT_WORD: &str = "print";

pub fn lower(node: Node) -> Node {
	let declared = declared_types(&node);
	let (printable, iterable) = (defining(&node, &declared, TEXT_OPERATION), defining(&node, &declared, ITERATE_OPERATION));
	if printable.is_empty() && iterable.is_empty() {
		return node;
	}
	let types = InstanceTypes::of(&node);
	Operations { printable, iterable, types }.rewrite_program(node)
}

struct Operations {
	/// The declared types with a `text(p:T)` definition, and with an `iterate(p:T)` one
	printable: HashSet<String>,
	iterable: HashSet<String>,
	types: InstanceTypes,
}

impl Operations {
	/// The program's last statement, when it is an instance of a printable type, is its text (card instance-final):
	/// a program shows the value `str()` would
	fn rewrite_program(&self, node: Node) -> Node {
		match node {
			Node::Meta { node, data } if is_statements(&node) => Node::Meta { node: Box::new(self.rewrite_program(*node)), data },
			Node::List(mut statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
				let last = statements.pop().map(|last| self.rewrite_program(last));
				let statements = statements.into_iter().map(|statement| self.rewrite(statement)).chain(last).collect();
				Node::List(statements, Bracket::None, separator)
			}
			value => self.call(TEXT_OPERATION, &self.printable, &value).unwrap_or_else(|| self.rewrite(value)),
		}
	}

	fn rewrite(&self, node: Node) -> Node {
		for (operation, types) in [(TEXT_OPERATION, &self.printable), (ITERATE_OPERATION, &self.iterable)] {
			if let Some((type_name, parameter, body)) = definition(&node, types, operation) {
				let head = call(&witness_name(operation, &type_name), vec![parameter]);
				let body = self.types.in_definition(&head, || self.rewrite(body));
				return key(head, Op::Define, body);
			}
		}
		match node {
			Node::Key(value, Op::As, target) if is_text_word(&target) => match self.call(TEXT_OPERATION, &self.printable, &value) {
				Some(call) => call,
				None => Node::Key(Box::new(self.rewrite(*value)), Op::As, target),
			},
			Node::List(items, bracket, separator) => {
				let text_call = |value: &Node| self.call(TEXT_OPERATION, &self.printable, value);
				let items_of = |value: &Node| self.call(ITERATE_OPERATION, &self.iterable, value);
				let called = match items.as_slice() {
					[word, value] if is_text_word(word) || word.name() == crate::wasm_emitter::text_builtins::TEXT_FORM => text_call(value),
					[word, value] if word.name() == PRINT_WORD => text_call(value).map(|text| Node::List(vec![word.clone(), text], bracket.clone(), separator.clone())),
					// `for x in b body`, `x in b`: the items iterate gives
					[word, variable, keyword, iterated, rest @ ..] if word.name() == FOR_WORD && keyword.name() == IN_WORD => items_of(iterated).map(|walked| {
						let rest: Vec<Node> = rest.iter().map(|item| self.rewrite(item.clone())).collect();
						Node::List([vec![word.clone(), variable.clone(), keyword.clone(), walked], rest].concat(), bracket.clone(), separator.clone())
					}),
					[value, keyword, searched] if keyword.name() == IN_WORD => items_of(searched)
						.map(|walked| Node::List(vec![self.rewrite(value.clone()), keyword.clone(), walked], bracket.clone(), separator.clone())),
					_ => None,
				};
				called.unwrap_or_else(|| Node::List(items.into_iter().map(|item| self.rewrite(item)).collect(), bracket, separator))
			}
			Node::Key(head, Op::Define, body) => {
				let body = self.types.in_definition(&head, || self.rewrite(*body));
				Node::Key(head, Op::Define, Box::new(body))
			}
			Node::Key(left, op, right) => key(self.rewrite(*left), op, self.rewrite(*right)),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.rewrite(*node)), data },
			other => other,
		}
	}

	/// `text·person(x)`, `iterate·bag(b)` for a value known to be an instance of a type defining the operation
	fn call(&self, operation: &str, types: &HashSet<String>, value: &Node) -> Option<Node> {
		let Some(Shape::Instance(type_name)) = self.types.shape(value) else { return None };
		types.contains(&type_name).then(|| {
			call(&witness_name(operation, &type_name), vec![self.rewrite(value.clone())])
		})
	}
}

fn is_statements(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(_, Bracket::None, Separator::Semicolon | Separator::Newline))
}

pub(crate) fn is_text_word(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if TEXT_WORDS.contains(&word.as_str()))
}

fn declared_types(node: &Node) -> HashSet<String> {
	let mut types = HashSet::new();
	node.visit(&mut |part| if let Node::Type { name, .. } = part { types.insert(name.name()); });
	types
}

/// The declared types with a definition of `operation`, or with its witness already (a class's `text()` method is
/// `text·V` by now)
fn defining(node: &Node, declared: &HashSet<String>, operation: &str) -> HashSet<String> {
	let mut types = HashSet::new();
	node.visit(&mut |part| {
		if let Some((type_name, _, _)) = definition(part, declared, operation) {
			types.insert(type_name);
		}
		if let Some(type_name) = declared.iter().find(|type_name| defines_function(part, &witness_name(operation, type_name))) {
			types.insert(type_name.clone());
		}
	});
	types
}

fn defines_function(node: &Node, name: &str) -> bool {
	let Node::Key(head, Op::Define | Op::Assign, _) = node else { return false };
	matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().is_some_and(|word| word.name() == name))
}

/// `operation(p:T) := body` of a declared type T: T, the parameter and the body
fn definition(node: &Node, declared: &HashSet<String>, operation: &str) -> Option<(String, Node, Node)> {
	let Node::Key(head, Op::Define | Op::Assign, body) = node.drop_meta() else { return None };
	let Node::List(items, Bracket::Round, _) = head.drop_meta() else { return None };
	let [word, parameter] = items.as_slice() else { return None };
	let Node::Key(_, Op::Colon, type_node) = parameter.drop_meta() else { return None };
	let type_name = type_node.drop_meta().name();
	(word.name() == operation && declared.contains(&type_name)).then(|| (type_name, parameter.clone(), body.as_ref().clone()))
}
