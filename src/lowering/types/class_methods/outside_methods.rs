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
					let unshadowed = class_fields.iter().filter(|field| !parameters.contains(field)).collect();
					receiver_reads(body, &Readable { receiver: &receiver, fields: unshadowed, ..Readable::default() })
				}
				None => body,
			};
			Node::Key(head, op, Box::new(body))
		}
		other => other.map_children(|child| with_receiver_reads(child, fields)),
	}
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
		Node::List(items, Bracket::Round, _) => items[1..].iter().map(crate::lowering::nodes::parameter_word).collect(),
		_ => vec![],
	}
}
