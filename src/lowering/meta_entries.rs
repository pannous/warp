//! Meta information on objects (user decision 2026-10-03, wiki/meta.md): an annotation `@unit("cm") {x:1}` becomes the
//! meta entry `{x:1 @unit:"cm"}`, the form that survives emission: the runtime reads it like a field (`p.@unit`), but
//! never counts it nor compares it (`is_meta_entry` in wasm_emitter/equality.rs).
//! Also hints `p.source` when p has both the field `source` and the meta entry `@source`: the field wins.
//! Under `use comments` (P114: off by default) a comment before a binding is its meta information (wiki/comments.md):
//! `x.@comment` is the comment, `x.meta` the map `{comment: "…"}`, unless x is an object with a field `meta`. Without
//! the pragma such a read is an error naming it.

use crate::node::{meta_entry, Bracket, Node, Separator, ATTRIBUTE_MARK};
use crate::operators::Op;

pub fn lower(node: Node) -> Node {
	hint_shadowed_meta(&node);
	let mut comments = vec![];
	binding_comments(&node, &mut comments);
	as_entries(if comments.is_empty() { node } else { comment_reads(node, &comments) })
}

const COMMENT_KEY: &str = "comment";
const META_WORD: &str = "meta";

/// The comment a Meta layer of `node` carries (the parser puts it on the first word of a statement)
fn comment_of(node: &Node) -> Option<&str> {
	match node {
		Node::Meta { node, data } => match data.as_ref() {
			Node::Key(key, _, value) if matches!(key.as_ref(), Node::Symbol(name) if name == COMMENT_KEY) => match value.drop_meta() {
				Node::Text(comment) => Some(comment),
				_ => comment_of(node),
			},
			_ => comment_of(node),
		},
		_ => None,
	}
}

/// A commented binding `x: …`, `x = …`, `x := …`
struct Commented {
	name: String,
	comment: String,
	/// x is an object with a field `meta`, which `x.meta` reads
	meta_field: bool,
}

fn binding_comments(node: &Node, bindings: &mut Vec<Commented>) {
	let mut add = |target: &Node, comment: &str, value: &Node| bindings.push(Commented {
		name: target.drop_meta().name(),
		comment: comment.to_string(),
		meta_field: matches!(value.drop_meta(), Node::List(items, Bracket::Curly, _) if items.iter().any(|item| matches!(item.drop_meta(), Node::Key(key, _, _) if key.drop_meta().name() == META_WORD))),
	});
	match node {
		Node::Meta { node: inner, .. } => match (comment_of(node), inner.drop_meta()) {
			(Some(comment), Node::Key(target, Op::Colon | Op::Assign | Op::Define, value)) if matches!(target.drop_meta(), Node::Symbol(_)) => add(target, comment, value),
			_ => binding_comments(inner, bindings),
		},
		Node::Key(target, Op::Colon | Op::Assign | Op::Define, value) => match comment_of(target) {
			Some(comment) if matches!(target.drop_meta(), Node::Symbol(_)) => add(target, comment, value),
			_ => binding_comments(value, bindings),
		},
		Node::List(items, _, _) => items.iter().for_each(|item| binding_comments(item, bindings)),
		_ => {}
	}
}

/// `x.@comment` → the comment, `x.meta` → `{comment: "…"}` of a commented binding x (the last binding of a name)
fn comment_reads(node: Node, bindings: &[Commented]) -> Node {
	let binding = |name: &str| bindings.iter().rev().find(|binding| binding.name == name);
	match node {
		Node::Key(left, Op::Dot, right) => {
			let read = match (left.drop_meta(), right.drop_meta()) {
				(Node::Symbol(name), Node::Symbol(field)) => binding(name).and_then(|binding| {
					let comment = Node::Text(binding.comment.clone());
					let read = match field.as_str() {
						_ if field.strip_prefix(ATTRIBUTE_MARK) == Some(COMMENT_KEY) => comment,
						META_WORD if !binding.meta_field => Node::List(vec![Node::Key(Box::new(Node::Symbol(COMMENT_KEY.to_string())), Op::Colon, Box::new(comment))], Bracket::Curly, Separator::Space),
						_ => return None,
					};
					Some(match crate::diagnostic::comments_as_meta() {
						true => read,
						false => crate::node::error(&format!("no field {field}: {name}'s comment becomes its meta information under `use comments`")),
					})
				}),
				_ => None,
			};
			read.unwrap_or_else(|| Node::Key(Box::new(comment_reads(*left, bindings)), Op::Dot, Box::new(comment_reads(*right, bindings))))
		}
		Node::Meta { node, data } => Node::Meta { node: Box::new(comment_reads(*node, bindings)), data },
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| comment_reads(item, bindings)).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(comment_reads(*left, bindings)), op, Box::new(comment_reads(*right, bindings))),
		other => other,
	}
}

fn as_entries(node: Node) -> Node {
	match node {
		Node::Meta { node, data } => match annotation_entry(&data) {
			Some(entry) => with_entry(as_entries(*node), entry).unwrap_or_else(|inner| Node::Meta { node: Box::new(inner), data }),
			None => Node::Meta { node: Box::new(as_entries(*node)), data },
		},
		Node::List(items, Bracket::Curly, separator) => Node::List(meta_last(items.into_iter().map(as_entries).collect()), Bracket::Curly, separator),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(as_entries).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(as_entries(*left)), op, Box::new(as_entries(*right))),
		other => other,
	}
}

/// The fields in their order, then the meta entries: `{x:1 @s:2 y:3}` → `{x:1 y:3 @s:2}`.
/// A walk by position then never meets a meta entry before the last field (list_ops emit_list_walk).
fn meta_last(items: Vec<Node>) -> Vec<Node> {
	let (meta, fields): (Vec<Node>, Vec<Node>) = items.into_iter().partition(|item| meta_entry(item).is_some());
	fields.into_iter().chain(meta).collect()
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
				crate::normalize::set_position_of(part);
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

