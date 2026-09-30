//! `P(1, 2)` for a declared type `type P{x:int y:int}` constructs the instance `P{x:1 y:2}`.

use crate::analyzer::{call_name, collect_all_types};
use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::TypeRegistry;

pub fn lower(node: Node) -> Node {
	let mut registry = TypeRegistry::new();
	collect_all_types(&mut registry, &node);
	if registry.types().is_empty() {
		return node;
	}
	construct(node, &registry)
}

fn construct(node: Node, registry: &TypeRegistry) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| construct(item, registry)).collect();
			instance(&items, &bracket, &separator, registry).unwrap_or(Node::List(items, bracket, separator))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(construct(*left, registry)), op, Box::new(construct(*right, registry))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(construct(*node, registry)), data },
		other => other,
	}
}

/// The instance a call of a declared type constructs; a wrong argument count is an error value at the call
fn instance(items: &[Node], bracket: &Bracket, separator: &Separator, registry: &TypeRegistry) -> Option<Node> {
	let name = call_name(items, bracket, separator)?;
	let type_def = registry.get_by_name(name)?;
	let arguments = &items[1..];
	let required = type_def.fields.iter().filter(|field| !field.is_optional()).count();
	if arguments.len() < required || arguments.len() > type_def.fields.len() {
		let message = if required == type_def.fields.len() {
			format!("{name} takes {} fields, got {}", type_def.fields.len(), arguments.len())
		} else {
			format!("{name} takes {required} to {} fields, got {}", type_def.fields.len(), arguments.len())
		};
		return Some(Diagnostic::at(&items[0], message).into_error());
	}
	let fields = type_def
		.fields
		.iter()
		.zip(arguments.iter().chain(std::iter::repeat(&Node::Empty)))
		.map(|(field, value)| Node::Key(Box::new(Node::Symbol(field.name.clone())), Op::Colon, Box::new(value.clone())))
		.collect();
	let body = Node::List(fields, Bracket::Curly, Separator::Space);
	Some(Node::Key(Box::new(Node::Symbol(name.to_string())), Op::Colon, Box::new(body)))
}
