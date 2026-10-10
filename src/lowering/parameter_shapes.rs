//! A parameter shaped as an object (wiki/argument.md): `f(p{name, title?}) := …`, `to call person{name?, phone text,
//! mutable status} do …`. The shape lists the fields the function reads: `name?` is optional, `phone text` (or
//! `phone: text`) is the field `phone` of type text (P164), `mutable status` may be assigned. The parameter itself is
//! untyped; an object written at a call (`f({name:"Dick"})`) that lacks a required field, or holds a literal of
//! another type than its field's, is a compile error.

use super::nodes::key;
use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::Kind;
use std::collections::HashMap;

const OPTIONAL_MARK: char = '?';
/// Words before a field name that say how it may be used, not what it is called
const FIELD_MODIFIERS: [&str; 4] = ["mutable", "constant", "required", "optional"];

struct Field {
	name: String,
	optional: bool,
	type_word: Option<String>,
}

/// The fields of each shaped parameter of a function, by position (None: a plain parameter)
type Shapes = HashMap<String, Vec<Option<Vec<Field>>>>;

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
			shapes.insert(name.clone(), parameters.iter().map(|parameter| shape(parameter).map(|fields| fields.iter().filter_map(field).collect())).collect());
			let plain = parameters.into_iter().map(|parameter| match shape(&parameter) {
				Some(_) => parameter_name(&parameter),
				None => parameter,
			});
			let head = Node::List(std::iter::once(Node::Symbol(name)).chain(plain).collect(), Bracket::Round, Separator::None);
			key(head, op, unshaped(*body, shapes))
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
		Node::List(fields, Bracket::Curly, _) if fields.iter().all(|written| field(written).is_some()) => Some(fields),
		_ => None,
	}
}

fn parameter_name(parameter: &Node) -> Node {
	match parameter.drop_meta() {
		Node::Key(name, Op::Colon, _) => name.as_ref().clone(),
		other => other.clone(),
	}
}

/// `name?`, `phone text`, `phone: text`, `mutable status`
fn field(written: &Node) -> Option<Field> {
	let named = |word: &str, type_word: Option<String>| match word.strip_suffix(OPTIONAL_MARK) {
		Some(name) => Field { name: name.to_string(), optional: true, type_word },
		None => Field { name: word.to_string(), optional: false, type_word },
	};
	match written.drop_meta() {
		Node::Symbol(word) => Some(named(word, None)),
		Node::Key(name, Op::Colon, kind) => match (name.drop_meta(), kind.drop_meta()) {
			(Node::Symbol(name), Node::Symbol(kind)) => Some(named(name, Some(kind.clone()))),
			_ => None,
		},
		Node::List(words, _, _) => {
			let words: Vec<&Node> = words.iter().skip_while(|word| matches!(word.drop_meta(), Node::Symbol(word) if FIELD_MODIFIERS.contains(&word.as_str()))).collect();
			match words.as_slice() {
				[single] => field(single),
				[name, kind] => match (name.drop_meta(), kind.drop_meta()) {
					(Node::Symbol(name), Node::Symbol(kind)) => Some(named(name, Some(kind.clone()))),
					_ => None,
				},
				_ => None,
			}
		}
		_ => None,
	}
}

/// Calls of shaped functions with an object written in place: a missing required field or a literal of the wrong type is an error
fn checked_calls(node: Node, shapes: &Shapes) -> Node {
	if let Some(message) = refused_argument(&node, shapes) {
		return Diagnostic::at(&node, message).into_error();
	}
	node.map_children(|child| checked_calls(child, shapes))
}

fn refused_argument(call: &Node, shapes: &Shapes) -> Option<String> {
	let Node::List(items, Bracket::Round | Bracket::None, _) = call.drop_meta() else { return None };
	let (name, arguments) = items.split_first()?;
	let Node::Symbol(name) = name.drop_meta() else { return None };
	shapes.get(name)?.iter().zip(constructions(arguments)).find_map(|(fields, argument)| {
		let given = object_entries(&argument)?;
		fields.as_ref()?.iter().find_map(|field| match given.iter().find(|(key, _)| *key == field.name) {
			None if !field.optional => Some(format!("missing argument field {}", field.name)),
			None => None,
			Some((_, value)) => wrong_type(field, value),
		})
	})
}

/// `phone:"899-573-5842"` for the field `phone number`: the type error
fn wrong_type(field: &Field, value: &Node) -> Option<String> {
	let type_word = field.type_word.as_ref()?;
	let wanted = crate::analyzer::type_word_kind(type_word)?;
	let given = crate::analyzer::argument_literal_kind(value)?;
	let is_number = |kind: Kind| kind.is_int() || kind.is_float();
	// `"8"` is a one-character literal, a text all the same
	let fits = given == wanted || is_number(wanted) && is_number(given) && !(wanted.is_int() && given.is_float()) || wanted == Kind::Text && given == Kind::Codepoint;
	(!fits).then(|| format!("type error: field {} is {type_word}, got {}", field.name, value.serialize()))
}

/// The arguments of a braceless call `call person {name:"Dick"}`: a word before an object constructs it, one argument
fn constructions(arguments: &[Node]) -> Vec<Node> {
	let mut merged: Vec<Node> = Vec::new();
	for argument in arguments {
		let constructs = matches!(argument.drop_meta(), Node::List(_, Bracket::Curly, _)) && merged.last().is_some_and(|kind| matches!(kind.drop_meta(), Node::Symbol(_)));
		match constructs {
			true => {
				let kind = merged.pop().expect("guarded");
				merged.push(Node::List(vec![kind, argument.clone()], Bracket::None, Separator::Space));
			}
			false => merged.push(argument.clone()),
		}
	}
	merged
}

/// The entries of an object literal `{name:"Dick"}`, also constructed `person{name:"Dick"}` or tagged `Person:{…}`
fn object_entries(node: &Node) -> Option<Vec<(String, Node)>> {
	match node.drop_meta() {
		Node::List(entries, Bracket::Curly, _) => entries.iter().map(|entry| match entry.drop_meta() {
			Node::Key(key, Op::Colon, value) => Some((key.name(), value.as_ref().clone())),
			_ => None,
		}).collect(),
		Node::List(items, Bracket::None | Bracket::Round, _) => match items.as_slice() {
			[kind, object] if matches!(kind.drop_meta(), Node::Symbol(_)) => object_entries(object),
			_ => None,
		},
		Node::Key(kind, Op::Colon | Op::None, object) if matches!(kind.drop_meta(), Node::Symbol(_)) => object_entries(object),
		_ => None,
	}
}
