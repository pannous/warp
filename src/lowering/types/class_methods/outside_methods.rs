//! One method form (card one-method-form, notes/dispatch.md): an outside `def x(p:P, k) := …` is P's method as much
//! as a class-body `def x(k) := …`, which lowers to `x(self:P, k)`: in its body a bare field name of P reads p, as
//! in a class body
use super::*;
use std::collections::HashMap;

/// `def x(p:P) := n + 1` of a class P with a field n: `def x(p:P) := p.n + 1`
pub(super) fn outside_receiver_reads(node: Node) -> Node {
	let fields: HashMap<String, Vec<String>> =
		declared_layouts(&node).into_iter().map(|(class, layout)| (class, layout.fields)).collect();
	if fields.values().all(Vec::is_empty) {
		return node;
	}
	with_receiver_reads(node, &fields)
}

fn with_receiver_reads(node: Node, fields: &HashMap<String, Vec<String>>) -> Node {
	match node {
		Node::Type { .. } => node,
		Node::Key(head, op @ (Op::Assign | Op::Define), body) => {
			let body = with_receiver_reads(*body, fields);
			let receiver_fields =
				first_parameter(&head).and_then(|(receiver, class)| Some((receiver, fields.get(&class)?)));
			let body = match receiver_fields {
				Some((receiver, class_fields)) => {
					let parameters = parameter_names(&head);
					let unshadowed: Vec<&String> =
						class_fields.iter().filter(|field| !parameters.contains(field)).collect();
					for (write, field) in bare_field_writes(&body, &unshadowed) {
						if let Err(error) = ask_field_or_local(&write, &field, &receiver, &leading_name(&head)) {
							return error;
						}
					}
					receiver_reads(body, &Readable { receiver: &receiver, fields: unshadowed, ..Readable::default() })
				}
				None => body,
			};
			Node::Key(head, op, Box::new(body))
		}
		other => other.map_children(|child| with_receiver_reads(child, fields)),
	}
}

const FIELD_OR_LOCAL: &str = "field-or-local";

/// The first `n = …` (`n += …`) of each field n the body writes by its bare name, unless a `let n` makes n a local
fn bare_field_writes(body: &Node, fields: &[&String]) -> Vec<(Node, String)> {
	let mut writes: Vec<(Node, String)> = vec![];
	let mut locals = vec![];
	body.visit(&mut |part| {
		if let Some(local) = let_binding(part) {
			locals.push(local);
		}
		if let Node::Key(target, op, _) = part.drop_meta() {
			let name = target.drop_meta().symbol_name().unwrap_or_default().to_string();
			let is_field = fields.iter().any(|field| **field == name) && !locals.contains(&name);
			if op.is_write() && is_field && !writes.iter().any(|(_, written)| *written == name) {
				writes.push((part.clone(), name));
			}
		}
	});
	writes
}

/// `n = …` in an outside method `x(p:P)` whose class has a field n: the field p.n (the default, as in a class body) or a
/// new local? The text of the method shows no field n, so the reading is said
fn ask_field_or_local(write: &Node, field: &str, receiver: &str, function: &str) -> Result<usize, Node> {
	use crate::errors::diagnostic::{ask, reading, Ask, Fallback};
	let question =
		format!("does `{field} = …` inside {function} change the field {receiver}.{field}, or make a new local?");
	let readings = vec![
		reading(&format!("the field {receiver}.{field}"), &format!("{receiver}.{field} = …"))
			.replacing(format!("{field} ="), format!("{receiver}.{field} =")),
		reading(&format!("a new local of {function}"), &format!("let {field} = …"))
			.replacing(format!("{field} ="), format!("let {field} =")),
	];
	ask(&Ask::new(FIELD_OR_LOCAL, question, readings, Fallback::Warning)
		.written(&format!("{field} = …"))
		.at_node(write))
}

/// `(p, P)` of the head `x(p:P, …)`, also with a result type `x(p:P): int`
pub(super) fn first_parameter(head: &Node) -> Option<(String, String)> {
	match head.drop_meta() {
		Node::Key(call, Op::Colon, _) => first_parameter(call),
		Node::List(items, Bracket::Round, _) => match items.get(1)?.drop_meta() {
			Node::Key(receiver, Op::Colon, class) => {
				Some((receiver.drop_meta().symbol_name()?.to_string(), class.drop_meta().symbol_name()?.to_string()))
			}
			_ => None,
		},
		_ => None,
	}
}

/// The parameter names of a definition head `x(p:P, k, m = 1)`
fn parameter_names(head: &Node) -> Vec<String> {
	match head.drop_meta() {
		Node::Key(call, Op::Colon, _) => parameter_names(call),
		Node::List(items, Bracket::Round, _) => items[1..]
			.iter()
			.map(|parameter| match parameter.drop_meta() {
				Node::Key(name, Op::Colon | Op::Assign, _) => name.drop_meta().name(),
				other => other.name(),
			})
			.collect(),
		_ => vec![],
	}
}
