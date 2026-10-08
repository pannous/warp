//! The operations a declared type defines for itself, used where an instance of it is known (InstanceTypes):
//! Printable (user decision P31): `text(p:person) := …` gives the text of a person, the one function a type word may
//! name; `x as text`, `str(x)`, `text(x)`, `print x` and an interpolation hole `"\(x)"` call it.
//! Iterable (wiki/trait.md): `iterate(b:bag) := …` gives what a bag holds; `for x in b` walks it and `x in b` searches it.
//! Each definition becomes its witness (`text·person`, `iterate·bag`); every other value keeps its own text and items.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::traits::{witness_name, InstanceTypes, Shape};
use std::collections::HashSet;

const TEXT_OPERATION: &str = "text";
const ITERATE_OPERATION: &str = "iterate";
/// The words whose call or cast gives a value's text
const TEXT_WORDS: [&str; 4] = ["text", "string", "str", "String"];
const PRINT_WORD: &str = "print";
const FOR_WORD: &str = "for";
const IN_WORD: &str = "in";

pub fn lower(node: Node) -> Node {
	let declared = declared_types(&node);
	let (printable, iterable) = (defining(&node, &declared, TEXT_OPERATION), defining(&node, &declared, ITERATE_OPERATION));
	if printable.is_empty() && iterable.is_empty() {
		return node;
	}
	let types = InstanceTypes::of(&node);
	Operations { printable, iterable, types }.rewrite(node)
}

struct Operations {
	/// The declared types with a `text(p:T)` definition, and with an `iterate(p:T)` one
	printable: HashSet<String>,
	iterable: HashSet<String>,
	types: InstanceTypes,
}

impl Operations {
	fn rewrite(&self, node: Node) -> Node {
		for (operation, types) in [(TEXT_OPERATION, &self.printable), (ITERATE_OPERATION, &self.iterable)] {
			if let Some((type_name, parameter, body)) = definition(&node, types, operation) {
				let head = Node::List(vec![Node::Symbol(witness_name(operation, &type_name)), parameter], Bracket::Round, Separator::None);
				return Node::Key(Box::new(head), Op::Define, Box::new(self.rewrite(body)));
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
			Node::Key(left, op, right) => Node::Key(Box::new(self.rewrite(*left)), op, Box::new(self.rewrite(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.rewrite(*node)), data },
			other => other,
		}
	}

	/// `text·person(x)`, `iterate·bag(b)` for a value known to be an instance of a type defining the operation
	fn call(&self, operation: &str, types: &HashSet<String>, value: &Node) -> Option<Node> {
		let Some(Shape::Instance(type_name)) = self.types.shape(value) else { return None };
		types.contains(&type_name).then(|| {
			Node::List(vec![Node::Symbol(witness_name(operation, &type_name)), self.rewrite(value.clone())], Bracket::Round, Separator::None)
		})
	}
}

pub(crate) fn is_text_word(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if TEXT_WORDS.contains(&word.as_str()))
}

fn declared_types(node: &Node) -> HashSet<String> {
	let mut types = HashSet::new();
	node.visit(&mut |part| if let Node::Type { name, .. } = part { types.insert(name.name()); });
	types
}

/// The declared types with a definition of `operation`
fn defining(node: &Node, declared: &HashSet<String>, operation: &str) -> HashSet<String> {
	let mut types = HashSet::new();
	node.visit(&mut |part| if let Some((type_name, _, _)) = definition(part, declared, operation) { types.insert(type_name); });
	types
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
