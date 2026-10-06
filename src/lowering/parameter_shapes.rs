//! A parameter shaped as an object (wiki/argument.md): `f(p{name, title?}) := …`, `to call person{name?, phone number,
//! mutable status} do …`. The shape lists the fields the function reads: `name?` is optional, `phone number` is the
//! field `phone` (its type word is not checked yet), `mutable status` may be assigned. The parameter itself is untyped;
//! an object written at a call (`f({name:"Dick"})`) that lacks a required field is a compile error.

use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::HashMap;

const OPTIONAL_MARK: char = '?';
/// Words before a field name that say how it may be used, not what it is called
const FIELD_MODIFIERS: [&str; 4] = ["mutable", "constant", "required", "optional"];

/// The required fields of each shaped parameter of a function, by position
type Shapes = HashMap<String, Vec<Option<Vec<String>>>>;

pub fn lower(node: Node) -> Node {
	let mut shapes = Shapes::new();
	let node = unshaped(node, &mut shapes);
	if shapes.is_empty() {
		return node;
	}
	checked_calls(node, &shapes)
}

/// Definitions with their shaped parameters made plain, the shapes collected
fn unshaped(node: Node, shapes: &mut Shapes) -> Node {
	match node {
		Node::Key(head, op @ (Op::Define | Op::Assign), body) if shaped_head(&head).is_some() => {
			let (name, parameters) = shaped_head(&head).expect("guarded");
			let required = parameters.iter().map(|parameter| shape(parameter).map(required_fields)).collect();
			shapes.insert(name.clone(), required);
			let plain = parameters.into_iter().map(|parameter| match shape(&parameter) {
				Some(_) => parameter_name(&parameter),
				None => parameter,
			});
			let head = Node::List(std::iter::once(Node::Symbol(name)).chain(plain).collect(), Bracket::Round, crate::node::Separator::None);
			Node::Key(Box::new(head), op, Box::new(unshaped(*body, shapes)))
		}
		other => other.map_children(|child| unshaped(child, shapes)),
	}
}

/// `(f p:{…} …)`: the function's name and its parameters, when one of them is shaped
fn shaped_head(head: &Node) -> Option<(String, Vec<Node>)> {
	let Node::List(items, Bracket::Round, _) = head.drop_meta() else { return None };
	let (name, parameters) = items.split_first()?;
	let Node::Symbol(name) = name.drop_meta() else { return None };
	parameters.iter().any(|parameter| shape(parameter).is_some()).then(|| (name.clone(), parameters.to_vec()))
}

/// The fields of `p:{name, title?}`
fn shape(parameter: &Node) -> Option<&[Node]> {
	let Node::Key(_, Op::Colon, kind) = parameter.drop_meta() else { return None };
	match kind.drop_meta() {
		Node::List(fields, Bracket::Curly, _) if fields.iter().all(|field| field_name(field).is_some()) => Some(fields),
		_ => None,
	}
}

fn parameter_name(parameter: &Node) -> Node {
	match parameter.drop_meta() {
		Node::Key(name, Op::Colon, _) => name.as_ref().clone(),
		other => other.clone(),
	}
}

/// `name?` → (name, optional), `phone number` → phone, `mutable status` → status
fn field_name(field: &Node) -> Option<(String, bool)> {
	match field.drop_meta() {
		Node::Symbol(word) => Some(match word.strip_suffix(OPTIONAL_MARK) {
			Some(name) => (name.to_string(), true),
			None => (word.clone(), false),
		}),
		Node::List(words, _, _) => {
			let mut words = words.iter().skip_while(|word| matches!(word.drop_meta(), Node::Symbol(word) if FIELD_MODIFIERS.contains(&word.as_str())));
			field_name(words.next()?)
		}
		_ => None,
	}
}

fn required_fields(fields: &[Node]) -> Vec<String> {
	fields.iter().filter_map(field_name).filter(|(_, optional)| !optional).map(|(name, _)| name).collect()
}

/// Calls of shaped functions with an object written in place: the first missing required field is an error
fn checked_calls(node: Node, shapes: &Shapes) -> Node {
	if let Some(missing) = missing_field(&node, shapes) {
		return Diagnostic::at(&node, format!("missing argument field {missing}")).into_error();
	}
	node.map_children(|child| checked_calls(child, shapes))
}

fn missing_field(call: &Node, shapes: &Shapes) -> Option<String> {
	let Node::List(items, Bracket::Round | Bracket::None, _) = call.drop_meta() else { return None };
	let (name, arguments) = items.split_first()?;
	let Node::Symbol(name) = name.drop_meta() else { return None };
	let required = shapes.get(name)?;
	required.iter().zip(constructions(arguments)).find_map(|(fields, argument)| {
		let given = object_fields(&argument)?;
		fields.as_ref()?.iter().find(|field| !given.contains(field)).cloned()
	})
}

/// The arguments of a braceless call `call person {name:"Dick"}`: a word before an object constructs it, one argument
fn constructions(arguments: &[Node]) -> Vec<Node> {
	let mut merged: Vec<Node> = Vec::new();
	for argument in arguments {
		let constructs = matches!(argument.drop_meta(), Node::List(_, Bracket::Curly, _)) && merged.last().is_some_and(|kind| matches!(kind.drop_meta(), Node::Symbol(_)));
		match constructs {
			true => {
				let kind = merged.pop().expect("guarded");
				merged.push(Node::List(vec![kind, argument.clone()], Bracket::None, crate::node::Separator::Space));
			}
			false => merged.push(argument.clone()),
		}
	}
	merged
}

/// The field names of an object literal `{name:"Dick"}`, also constructed `person{name:"Dick"}` or tagged `Person:{…}`
fn object_fields(node: &Node) -> Option<Vec<String>> {
	match node.drop_meta() {
		Node::List(entries, Bracket::Curly, _) => entries.iter().map(|entry| match entry.drop_meta() {
			Node::Key(key, Op::Colon, _) => Some(key.name()),
			_ => None,
		}).collect(),
		Node::List(items, Bracket::None | Bracket::Round, _) => match items.as_slice() {
			[kind, object] if matches!(kind.drop_meta(), Node::Symbol(_)) => object_fields(object),
			_ => None,
		},
		Node::Key(kind, Op::Colon | Op::None, object) if matches!(kind.drop_meta(), Node::Symbol(_)) => object_fields(object),
		_ => None,
	}
}
