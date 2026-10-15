//! A parameter of type Type (card random-should, decision random-forms): `f(t: type) := t == int` takes the type
//! itself, so `f int`, `f(text)` and `f Circle` pass the type value a bare type word is, never its constructor's call. The
//! type value is a node at run time, held as `any` holds it; a literal argument that is no type is a loud error.

use super::memoization::definition_parts;
use super::nodes::{called_name, children_rewritten, function_head, key};
use crate::node::Node;
use crate::operators::Op;
use std::collections::HashMap;

/// The annotations naming the type of types: `t: type`, `t: Type`
const TYPE_ANNOTATIONS: [&str; 2] = [crate::type_tests::TYPE_WORD, "Type"];

pub fn lower(node: Node) -> Node {
	let typed = type_parameter_positions(&node);
	if typed.is_empty() {
		return node;
	}
	match misplaced_argument(&node, &typed) {
		Some(error) => error,
		None => held_as_any(node),
	}
}

fn is_type_annotation(node: &Node) -> bool {
	TYPE_ANNOTATIONS.iter().any(|word| node.is_symbol(word))
}

/// The positions of the parameters declared of type Type, by function name
fn type_parameter_positions(node: &Node) -> HashMap<String, Vec<usize>> {
	let mut typed = HashMap::new();
	node.visit(&mut |part| {
		let Some(head) = function_head(part) else { return };
		let positions: Vec<usize> = (head.written_parameters.iter().enumerate())
			.filter(|(_, parameter)| matches!(parameter.drop_meta(), Node::Key(_, Op::Colon, annotation) if is_type_annotation(annotation)))
			.map(|(position, _)| position)
			.collect();
		if !positions.is_empty() {
			typed.insert(head.name.to_string(), positions);
		}
	});
	typed
}

/// `f 3` of `f(t: type)`: the error naming what a type parameter takes
fn misplaced_argument(node: &Node, typed: &HashMap<String, Vec<usize>>) -> Option<Node> {
	let mut error = None;
	node.visit(&mut |part| {
		let Node::List(items, bracket, separator) = part.drop_meta() else { return };
		let Some(positions) = called_name(items, bracket, separator).and_then(|name| typed.get(name)) else { return };
		let literal = positions.iter().filter_map(|position| items.get(position + 1)).find(|argument| is_literal_value(argument));
		if let (None, Some(argument)) = (&error, literal) {
			error = Some(crate::node::error(&format!(
				"{}: the parameter of type type takes a type like int, text or a class name, not the value {}",
				items[0].drop_meta().name(),
				argument.drop_meta().serialize()
			)));
		}
	});
	error
}

fn is_literal_value(argument: &Node) -> bool {
	matches!(argument.drop_meta(), Node::Number(_) | Node::Text(_) | Node::Char(_))
}

/// `t: type` in a function head is `t: any`: the type value passes as the node it is
fn held_as_any(node: Node) -> Node {
	if let Some((head, body, rebuild)) = definition_parts(&node) {
		return rebuild(annotated_any(head), held_as_any(body));
	}
	children_rewritten(node, held_as_any)
}

fn annotated_any(head: Node) -> Node {
	match head {
		Node::Key(parameter, Op::Colon, annotation) if is_type_annotation(&annotation) => {
			key(*parameter, Op::Colon, Node::Symbol(crate::words::ANY_TYPE.to_string()))
		}
		Node::Meta { node, data } => Node::Meta { node: Box::new(annotated_any(*node)), data },
		other => children_rewritten(other, annotated_any),
	}
}
