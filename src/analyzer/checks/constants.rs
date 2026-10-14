//! Constants: `const` and `let` bind once; later changes are errors, the same value again only warns
use super::*;
use crate::words::LET_WORD;

/// Each bound name with the text of its value, and the value nodes
type Bindings<'a> = (Vec<(String, String)>, &'a [Node]);

/// `const x=…` binds x once: a later assignment of another value, a compound assignment, an increment or an element
/// assignment is rejected (P130); the same value again only warns (P159). `let x=…` is fully immutable: any change
/// of it is an error naming `var` (user, card let-reassign)
pub(crate) fn let_change(node: &Node, place: &str, change: &str) -> Diagnostic {
	Diagnostic::at(node, format!("{place} is let (immutable), cannot change it: {change}"))
		.fix(format!("use var {place}"))
}

/// The method call `add(4)`, `pop()`, `remove(x)` of `names.add(4)`: one that changes the list it is called on
pub(crate) fn changing_list_method(call: &Node) -> bool {
	let method = match call.drop_meta() {
		Node::List(items, _, _) => items.first().map(Node::name),
		_ => call.symbol_name().map(String::from),
	};
	method.is_some_and(|method| is_list_mutating_method(&method))
}

/// The names a `let`/`const` declaration binds, each with the text of its value, and the value nodes: `x = v`, or the
/// lowered tuple pattern `let (a, b) = v, w` (`$destructure (a, b) v w`, one value unpacked when there is only one)
pub(crate) fn declared_bindings(declaration: &Node) -> Option<Bindings<'_>> {
	if let Node::Key(target, Op::Assign | Op::Define, value) = declaration.drop_meta() {
		return Some((vec![(target.name(), value.serialize())], std::slice::from_ref(value)));
	}
	let (names, values) = crate::tuples::destructuring(declaration)?;
	let value_of = |index: usize| match values {
		[single] => format!("{}#{}", single.serialize(), index + 1),
		_ => values[index].serialize(),
	};
	Some((names.into_iter().enumerate().map(|(index, name)| (name, value_of(index))).collect(), values))
}

pub(crate) fn check_constants(node: &Node, constants: &mut HashMap<String, (String, String)>) -> Option<Diagnostic> {
	// a function's own `let`/`const` bind in it only; it still sees main's (card serve-var)
	if let Some(body) = super::variables::function_definition_body(node) {
		return check_constants(body, &mut constants.clone());
	}
	match node.drop_meta() {
		Node::List(items, _, _) => {
			let statements = match items.as_slice() {
				[keyword, declaration, rest @ ..] if is_constant_keyword(keyword) || keyword.is_symbol(LET_WORD) => {
					if let Some(found) = declaration_problem(&keyword.name(), declaration, constants) {
						return Some(found);
					}
					rest
				}
				_ => items.as_slice(),
			};
			statements.iter().find_map(|statement| check_constants(statement, constants))
		}
		Node::Key(target, op, value)
			if matches!(op, Op::Assign | Op::Define | Op::Inc | Op::Dec) || op.is_compound_assign() =>
		{
			reassignment_problem(node, target, op, value, constants).or_else(|| check_constants(value, constants))
		}
		// `names.add(4)`, `names.pop()` change the list as an assignment would (card const-list-add)
		Node::Key(list, Op::Dot, call) if changing_list_method(call) => {
			list_change_problem(node, &list.name(), constants).or_else(|| check_constants(call, constants))
		}
		Node::Key(left, _, right) => check_constants(left, constants).or_else(|| check_constants(right, constants)),
		_ => None,
	}
}

/// `const x = v`, `let x = v`: a declaration without a value, or a problem in its value; else its names are bound
fn declaration_problem(
	keyword: &str,
	declaration: &Node,
	constants: &mut HashMap<String, (String, String)>,
) -> Option<Diagnostic> {
	let Some((bindings, values)) = declared_bindings(declaration) else {
		return Some(
			Diagnostic::at(declaration, format!("{keyword} needs a value: {}", declaration.serialize()))
				.fix(format!("{keyword} x = 5")),
		);
	};
	if let Some(found) = values.iter().find_map(|value| check_constants(value, constants)) {
		return Some(found);
	}
	constants.extend(bindings.into_iter().map(|(name, value)| (name, (keyword.to_string(), value))));
	None
}

/// An assignment of a let or const: an error, unless it assigns a const its own value again (a warning)
fn reassignment_problem(
	node: &Node,
	target: &Node,
	op: &Op,
	value: &Node,
	constants: &HashMap<String, (String, String)>,
) -> Option<Diagnostic> {
	let place = match target.drop_meta() {
		Node::Key(name, Op::Hash, _) => name.name(),
		other => other.name(),
	};
	let assignment = node.drop_meta().serialize();
	match constants.get(&place)? {
		(keyword, _) if keyword == LET_WORD => Some(let_change(node, &place, &assignment)),
		(_, bound)
			if *op == Op::Assign && matches!(target.drop_meta(), Node::Symbol(_)) && value.serialize() == *bound =>
		{
			let redundant =
				Diagnostic::at(node, format!("{place} is const and already {bound}: {assignment} changes nothing"))
					.fix("remove the redundant assignment".to_string());
			crate::errors::diagnostic::report(std::slice::from_ref(&redundant)).is_err().then_some(redundant)
		}
		_ => Some(
			Diagnostic::at(node, format!("{place} is const, cannot assign it again: {assignment}"))
				.fix(format!("use a new name instead of {place}, or declare it without const")),
		),
	}
}

/// A list method changing a let or const list
fn list_change_problem(node: &Node, place: &str, constants: &HashMap<String, (String, String)>) -> Option<Diagnostic> {
	let change = node.drop_meta().serialize();
	match constants.get(place)? {
		(keyword, _) if keyword == LET_WORD => Some(let_change(node, place, &change)),
		_ => Some(
			Diagnostic::at(node, format!("{place} is const, cannot change it: {change}"))
				.fix(format!("change a copy (copy = {place}), or declare {place} without const")),
		),
	}
}
