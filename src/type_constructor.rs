//! `P(1, 2)` for a declared type `type P{x:int y:int}` constructs the instance `P{x:1 y:2}`.
//! `P{x:1 y:2}` (glued, parsed as the key `P` Op::None `{…}`) constructs and validates one too, while `P:{…}` is plain
//! data (D4): an instance is the data key marked `Instance`, emitted with its own op code, so it never equals the data.

use crate::analyzer::{builtin_type_kind, call_name, collect_all_types, literal_kind};
use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::{Kind, TypeDef, TypeRegistry};

/// Marks the data key `P:{…}` that a construction `P{…}` validated
#[derive(Clone, Debug, PartialEq)]
pub struct Instance;

/// The type name and fields of a constructed instance, behind any metadata
pub fn instance_parts(node: &Node) -> Option<(&Node, &Node)> {
	match node {
		Node::Meta { node, data } if matches!(data.as_ref(), Node::Data(dada) if dada.downcast_ref::<Instance>().is_some()) => match node.drop_meta() {
			Node::Key(name, Op::Colon, fields) => Some((name, fields)),
			_ => None,
		},
		Node::Meta { node, .. } => instance_parts(node),
		_ => None,
	}
}

pub fn lower(node: Node) -> Node {
	let mut registry = TypeRegistry::new();
	collect_all_types(&mut registry, &node);
	construct(node, &registry)
}

fn construct(node: Node, registry: &TypeRegistry) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| construct(item, registry)).collect();
			instance(&items, &bracket, &separator, registry).unwrap_or(Node::List(items, bracket, separator))
		}
		Node::Key(name, Op::None, fields) if matches!(name.drop_meta(), Node::Symbol(_)) => {
			let fields = Box::new(construct(*fields, registry));
			let data = Node::Key(name.clone(), Op::Colon, fields.clone());
			match registry.get_by_name(&name.name()) {
				Some(type_def) => match field_error(type_def, &name, &fields) {
					Some(error) => error,
					None => Node::meta(data, Node::data(Instance)),
				},
				None => data, // the pre-scan of the parser can mistake a word for a type name: plain data
			}
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
	Some(Node::meta(Node::Key(Box::new(Node::Symbol(name.to_string())), Op::Colon, Box::new(body)), Node::data(Instance)))
}

/// The first field of `P{…}` that P does not declare or whose literal value does not fit its declared type
fn field_error(type_def: &TypeDef, located: &Node, fields: &Node) -> Option<Node> {
	let name = &type_def.name;
	let entries = match fields.drop_meta() {
		Node::List(items, _, _) => items.clone(),
		Node::Empty => vec![],
		single => vec![single.clone()],
	};
	entries.iter().find_map(|entry| {
		let Node::Key(field, Op::Colon | Op::Assign, value) = entry.drop_meta() else {
			let message = format!("{name}{{…}} takes fields as name:value, got {}", entry.serialize());
			return Some(Diagnostic::at(located, message).into_error());
		};
		let field = field.name();
		let Some(declared) = type_def.fields.iter().find(|declared| declared.name == field) else {
			let message = format!("{name} has no field {field}");
			return Some(Diagnostic::at(located, message).fix(format!("declare {field} in {name}, or write the data {name}:{{…}}")).into_error());
		};
		let declared_type = declared.type_name.trim_end_matches('?');
		let expected = builtin_type_kind(declared_type)?;
		let actual = literal_kind(value)?;
		let fits = expected == actual || (expected == Kind::Float && actual == Kind::Int) || (expected == Kind::Text && actual == Kind::Codepoint);
		(!fits).then(|| {
			let message = format!("{name}.{field} is {declared_type}, got {} {}", format!("{actual:?}").to_lowercase(), value.serialize());
			Diagnostic::at(located, message).into_error()
		})
	})
}
