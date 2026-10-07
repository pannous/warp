//! Card bool-type (user, 2026-10-07): true and false are a shallow type of their own. They act as the Ints 1 and 0
//! everywhere, but `type(true)` is bool and they print as true/false. Which expressions are bools, statically.

use super::*;

pub const BOOL_TYPE: &str = "bool";
/// Calls whose value is a bool
const BOOL_CALLS: [&str; 2] = [crate::type_tests::IS_TYPE, crate::traits::INSTANCE_OF];

thread_local! {
	/// The user functions of the program being emitted whose result is a bool (note_bool_functions)
	static BOOL_FUNCTIONS: std::cell::RefCell<HashSet<String>> = std::cell::RefCell::new(HashSet::new());
}

/// `even(n) := n % 2 == 0`: the functions whose body ends in a bool, also through each other (`odd(n) := not even(n)`),
/// for the emission of one program
pub fn note_bool_functions<'a>(functions: impl Iterator<Item = &'a crate::context::UserFunctionDef> + Clone) {
	BOOL_FUNCTIONS.with(|known| known.borrow_mut().clear());
	loop {
		let found: Vec<String> = functions.clone().filter(|function| !is_bool_function(&function.name) && is_boolean(&function.body, &Scope::new())).map(|function| function.name.clone()).collect();
		if found.is_empty() {
			return;
		}
		BOOL_FUNCTIONS.with(|known| known.borrow_mut().extend(found));
	}
}

fn is_bool_function(name: &str) -> bool {
	BOOL_FUNCTIONS.with(|known| known.borrow().contains(name))
}

/// `true`, `3 > 2`, `not x`, `a and b` of bools, `x is int`, a variable bound to one, `(…; x < 3)` ending in one
pub fn is_boolean(node: &Node, scope: &Scope) -> bool {
	match node.drop_meta() {
		Node::True | Node::False => true,
		Node::Key(left, op, right) => match op {
			_ if op.is_comparison() => true,
			Op::Not => matches!(left.drop_meta(), Node::Empty),
			Op::And | Op::Or => is_boolean(left, scope) && is_boolean(right, scope),
			Op::Assign | Op::Define => matches!(left.drop_meta(), Node::Symbol(_)) && is_boolean(right, scope),
			_ => false,
		},
		Node::Symbol(name) => is_bool_variable(name, scope),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => is_boolean(&items[0], scope),
		Node::List(items, _, Separator::Semicolon | Separator::Newline) => items.last().is_some_and(|last| is_boolean(last, scope)),
		Node::List(items, _, _) => matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if BOOL_CALLS.contains(&name.as_str()) || is_bool_function(name)),
		_ => false,
	}
}

/// `x = 3 > 2`, `x: bool = …`: a variable holding a bool
pub fn is_bool_variable(name: &str, scope: &Scope) -> bool {
	scope.lookup(name).and_then(|local| local.type_node.as_deref()).is_some_and(|type_node| type_node.drop_meta().name() == BOOL_TYPE)
}
