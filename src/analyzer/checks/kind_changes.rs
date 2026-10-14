//! Kind changes: an assignment must keep the kind of its variable
use super::*;

/// Check for type errors in the AST, returns Some(Node::Error) if found
pub(crate) fn check_type_errors(node: &Node, scope: &mut Scope) -> Option<Node> {
	check_type_errors_inner(node, scope, false)
}

pub(crate) fn check_type_errors_inner(node: &Node, scope: &mut Scope, in_structure: bool) -> Option<Node> {
	let node = node.drop_meta();
	if let Some(body) = function_definition_body(node) {
		return check_type_errors_inner(body, &mut Scope::new(), false);
	}
	match node {
		Node::Key(left, Op::Colon, right) if left.is_symbol("global") => check_type_errors_inner(right, scope, false),
		// a tag `name: {…}` is structure on its right side
		Node::Key(left, Op::Colon, right) if left.symbol_name().is_some() => {
			check_type_errors_inner(left, scope, in_structure).or_else(|| check_type_errors_inner(right, scope, true))
		}
		Node::Key(left, op @ (Op::Define | Op::Assign), right) => {
			let checks_assignment = *op == Op::Define || !in_structure;
			let assignment_error = match left.symbol_name() {
				Some(name) if checks_assignment => check_assignment(name, right, scope),
				_ => None,
			};
			assignment_error.or_else(|| check_type_errors_inner(right, scope, in_structure))
		}
		Node::Key(left, _, right) => check_type_errors_inner(left, scope, in_structure)
			.or_else(|| check_type_errors_inner(right, scope, in_structure)),
		Node::List(items, _, _) => items
			.iter()
			.find_map(|item| check_type_errors_inner(item, scope, in_structure)),
		_ => None,
	}
}

/// `name = value` must keep the variable's kind; a new variable takes the value's kind
pub(crate) fn check_assignment(name: &str, value: &Node, scope: &mut Scope) -> Option<Node> {
	match scope.lookup(name) {
		Some(existing) => {
			let new_kind = written_kind(value, scope);
			(!types_compatible(existing.kind, new_kind)).then(|| type_error(name, existing.kind, new_kind))
		}
		None => {
			let kind = infer_type(value, scope);
			scope.define(name.to_string(), None, kind);
			None
		}
	}
}

pub(crate) fn type_error(name: &str, existing: Kind, new: Kind) -> Node {
	Node::Error(Box::new(Node::Text(format!(
		"type mismatch: cannot assign {} to variable '{}' of type {}",
		new, name, existing
	))))
}

/// Check if two types are compatible for assignment
pub(crate) fn types_compatible(existing: Kind, new: Kind) -> bool {
	match (existing, new) {
		// Same type is always compatible
		(a, b) if a == b => true,
		// Int and Float are NOT compatible (x=1; x=1.0 should fail)
		(Kind::Int, Kind::Float) | (Kind::Float, Kind::Int) => false,
		// Text and other types are NOT compatible
		(Kind::Text, _) | (_, Kind::Text) => false,
		// Codepoint and Int may be compatible (char as number)
		(Kind::Int, Kind::Codepoint) | (Kind::Codepoint, Kind::Int) => true,
		// Default: incompatible
		_ => false,
	}
}
