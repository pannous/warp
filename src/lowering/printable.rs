//! The Printable operation (user decision P31): `text(p:person) := …` gives the text of an instance of the declared type
//! person, the one function a type word may name. It becomes the witness `text·person`, and `x as text`, `str(x)`,
//! `text(x)`, `print x` and an interpolation hole `"\(x)"` call it where x is known to be an instance of person; every
//! other value keeps its own text.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::traits::{witness_name, InstanceTypes, Shape};
use std::collections::HashSet;

const TEXT_OPERATION: &str = "text";
/// The words whose call or cast gives a value's text
const TEXT_WORDS: [&str; 4] = ["text", "string", "str", "String"];
const PRINT_WORD: &str = "print";

pub fn lower(node: Node) -> Node {
	let printable = printable_types(&node);
	if printable.is_empty() {
		return node;
	}
	let types = InstanceTypes::of(&node);
	Printable { printable, types }.rewrite(node)
}

struct Printable {
	/// The declared types with a `text(p:T)` definition
	printable: HashSet<String>,
	types: InstanceTypes,
}

impl Printable {
	fn rewrite(&self, node: Node) -> Node {
		if let Some((type_name, parameter, body)) = printable_definition(&node, &self.printable) {
			let head = Node::List(vec![Node::Symbol(witness_name(TEXT_OPERATION, &type_name)), parameter], Bracket::Round, Separator::None);
			return Node::Key(Box::new(head), Op::Define, Box::new(self.rewrite(body)));
		}
		match node {
			Node::Key(value, Op::As, target) if is_text_word(&target) => match self.text_call(&value) {
				Some(call) => call,
				None => Node::Key(Box::new(self.rewrite(*value)), Op::As, target),
			},
			Node::List(items, bracket, separator) => {
				let called = match items.as_slice() {
					[word, value] if is_text_word(word) || word.name() == crate::wasm_emitter::text_builtins::TEXT_FORM => self.text_call(value),
					[word, value] if word.name() == PRINT_WORD => self.text_call(value).map(|text| Node::List(vec![word.clone(), text], bracket.clone(), separator.clone())),
					_ => None,
				};
				called.unwrap_or_else(|| Node::List(items.into_iter().map(|item| self.rewrite(item)).collect(), bracket, separator))
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.rewrite(*left)), op, Box::new(self.rewrite(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.rewrite(*node)), data },
			other => other,
		}
	}

	/// `text·person(x)` for a value known to be an instance of a printable type
	fn text_call(&self, value: &Node) -> Option<Node> {
		let Some(Shape::Instance(type_name)) = self.types.shape(value) else { return None };
		self.printable.contains(&type_name).then(|| {
			Node::List(vec![Node::Symbol(witness_name(TEXT_OPERATION, &type_name)), self.rewrite(value.clone())], Bracket::Round, Separator::None)
		})
	}
}

fn is_text_word(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if TEXT_WORDS.contains(&word.as_str()))
}

fn declared_types(node: &Node) -> HashSet<String> {
	let mut types = HashSet::new();
	node.visit(&mut |part| if let Node::Type { name, .. } = part { types.insert(name.name()); });
	types
}

fn printable_types(node: &Node) -> HashSet<String> {
	let declared = declared_types(node);
	let mut printable = HashSet::new();
	node.visit(&mut |part| if let Some((type_name, _, _)) = printable_definition(part, &declared) { printable.insert(type_name); });
	printable
}

/// `text(p:T) := body` of a declared type T: T, the parameter and the body
fn printable_definition(node: &Node, declared: &HashSet<String>) -> Option<(String, Node, Node)> {
	let Node::Key(head, Op::Define | Op::Assign, body) = node.drop_meta() else { return None };
	let Node::List(items, Bracket::Round, _) = head.drop_meta() else { return None };
	let [word, parameter] = items.as_slice() else { return None };
	let Node::Key(_, Op::Colon, type_node) = parameter.drop_meta() else { return None };
	let type_name = type_node.drop_meta().name();
	(word.name() == TEXT_OPERATION && declared.contains(&type_name)).then(|| (type_name, parameter.clone(), body.as_ref().clone()))
}
