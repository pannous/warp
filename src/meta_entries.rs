//! Meta information on objects (user decision 2026-10-03, wiki/meta.md): an annotation `@unit("cm") {x:1}` becomes the
//! meta entry `{x:1 @unit:"cm"}`, the form that survives emission: the runtime reads it like a field (`p.@unit`), but
//! never counts it nor compares it (`is_meta_entry` in wasm_emitter/equality.rs).
//! Also hints `p.source` when p has both the field `source` and the meta entry `@source`: the field wins.

use crate::node::{meta_entry, Bracket, Node, ATTRIBUTE_MARK};
use crate::operators::Op;

pub fn lower(node: Node) -> Node {
	hint_shadowed_meta(&node);
	as_entries(node)
}

fn as_entries(node: Node) -> Node {
	match node {
		Node::Meta { node, data } => match annotation_entry(&data) {
			Some(entry) => with_entry(as_entries(*node), entry).unwrap_or_else(|inner| Node::Meta { node: Box::new(inner), data }),
			None => Node::Meta { node: Box::new(as_entries(*node)), data },
		},
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(as_entries).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(as_entries(*left)), op, Box::new(as_entries(*right))),
		other => other,
	}
}

/// The annotation `@name(value)` a Meta layer carries, as the entry `@name:value`
fn annotation_entry(data: &Node) -> Option<Node> {
	let Node::Key(key, _, value) = data else { return None };
	let Node::Symbol(name) = key.as_ref() else { return None };
	name.starts_with(ATTRIBUTE_MARK).then(|| Node::Key(key.clone(), Op::Colon, value.clone()))
}

/// The object with the entry added to its fields; Err gives back anything that is no object
fn with_entry(object: Node, entry: Node) -> Result<Node, Node> {
	match object {
		Node::Meta { node, data } => with_entry(*node, entry).map(|node| Node::Meta { node: Box::new(node), data }),
		Node::List(mut items, Bracket::Curly, separator) => {
			items.push(entry);
			Ok(Node::List(items, Bracket::Curly, separator))
		}
		Node::Key(name, op @ (Op::Colon | Op::None), fields) if matches!(fields.drop_meta(), Node::List(_, Bracket::Curly, _)) => {
			with_entry(*fields, entry).map(|fields| Node::Key(name, op, Box::new(fields)))
		}
		other => Err(other),
	}
}

/// `p = {source:"field" @source:"gps"}; p.source` reads the field and hints `p.@source` for the meta entry
fn hint_shadowed_meta(node: &Node) {
	let mut objects: Vec<(String, Node)> = vec![];
	node.visit(&mut |part| match part {
		Node::Key(target, Op::Assign | Op::Define, value) if matches!(target.drop_meta(), Node::Symbol(_)) => {
			objects.push((target.name(), value.drop_meta().clone()));
		}
		Node::Key(object, Op::Dot, field) => {
			let (Node::Symbol(object), Node::Symbol(field)) = (object.drop_meta(), field.drop_meta()) else { return };
			let Some((_, value)) = objects.iter().rev().find(|(name, _)| name == object) else { return };
			let shadowed = value.meta_entries().any(|(name, _)| name == field) && has_field(value, field);
			if shadowed {
				let reason = format!("{object} has both the field {field} and the meta key @{field}: the field wins");
				crate::normalize::hint(&format!("{object}.{field}"), &format!("{object}.@{field}"), &reason);
			}
		}
		_ => {}
	});
}

fn has_field(object: &Node, field: &str) -> bool {
	let fields: &[Node] = match object.drop_meta() {
		Node::List(items, _, _) => items,
		Node::Key(_, _, value) => match value.drop_meta() {
			Node::List(items, _, _) => items,
			_ => return false,
		},
		_ => return false,
	};
	fields.iter().any(|entry| meta_entry(entry).is_none() && matches!(entry.drop_meta(), Node::Key(key, _, _) if key.name() == field))
}
