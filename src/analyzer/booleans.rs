//! Card bool-type (user, 2026-10-07): true and false are a shallow type of their own. They act as the Ints 1 and 0
//! everywhere, but `type(true)` is bool and they print as true/false. Which expressions are bools, statically.

use super::*;

pub const BOOL_TYPE: &str = "bool";
/// Calls whose value is a bool
const BOOL_CALLS: [&str; 7] = [crate::type_tests::IS_TYPE, crate::traits::INSTANCE_OF, crate::wasm_emitter::text_builtins::IS_ERROR, crate::library_words::VALUES_SIMILAR, crate::library_words::VALUES_ROUGH, crate::uncertain::CERTAINLY, crate::uncertain::POSSIBLY];

thread_local! {
	/// The user functions of the program being emitted whose result is a bool (note_bool_functions)
	static BOOL_FUNCTIONS: std::cell::RefCell<HashSet<String>> = std::cell::RefCell::new(HashSet::new());
}

/// `even(n) := n % 2 == 0`: the functions whose body ends in a bool, also through each other (`odd(n) := not even(n)`)
/// and themselves (`f(n) := if n == 0 then true else f(n - 1)`), for the emission of one program: every function is
/// taken for one until its body shows otherwise
pub fn note_bool_functions<'a>(functions: impl Iterator<Item = &'a crate::context::UserFunctionDef> + Clone) {
	BOOL_FUNCTIONS.with(|known| *known.borrow_mut() = functions.clone().map(|function| function.name.clone()).collect());
	loop {
		let refuted: Vec<String> = functions.clone().filter(|function| is_bool_function(&function.name) && !gives_bool(function)).map(|function| function.name.clone()).collect();
		if refuted.is_empty() {
			return;
		}
		BOOL_FUNCTIONS.with(|known| known.borrow_mut().retain(|name| !refuted.contains(name)));
	}
}

/// The body ends in a bool and every `return` in it returns one
fn gives_bool(function: &crate::context::UserFunctionDef) -> bool {
	let scope = parameter_scope(function);
	let mut returns_bool = true;
	function.body.visit(&mut |node| if let Node::List(items, _, _) = node.drop_meta() {
		if let [word, value] = items.as_slice() {
			returns_bool &= word.drop_meta().name() != crate::lowering::tuples::RETURN || is_boolean(value, &scope);
		}
	});
	returns_bool && is_boolean(&function.body, &scope)
}

/// The function's parameters declared bool (`f(b: bool) := b`), the rest unknown
fn parameter_scope(function: &crate::context::UserFunctionDef) -> Scope {
	let mut scope = Scope::new();
	for param in function.params.iter().filter(|param| param.annotation.as_ref().is_some_and(|annotation| annotation.drop_meta().name() == BOOL_TYPE)) {
		scope.define(param.name.clone(), param.annotation.clone().map(Box::new), Kind::Int);
	}
	scope
}

pub fn is_bool_function(name: &str) -> bool {
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
			// `c ? yes : f(n)`, `if c then yes else f(n)`: both branches
			Op::Question => matches!(right.drop_meta(), Node::Key(then, Op::Colon, otherwise) if is_boolean(then, scope) && is_boolean(otherwise, scope)),
			Op::Else => matches!(left.drop_meta(), Node::Key(_, Op::Then, then) if is_boolean(then, scope)) && is_boolean(right, scope),
			// `x: bool = 1` (P199): the value stored, a bool
			Op::Assign | Op::Define => matches!(left.drop_meta(), Node::Symbol(name) if is_bool_variable(name, scope) || is_boolean(right, scope)),
			_ => false,
		},
		Node::Symbol(name) => is_bool_variable(name, scope),
		// `(x < 3)`, a braced body `def ok(x){x < 10}`
		Node::List(items, Bracket::Round | Bracket::Curly, _) if items.len() == 1 && is_boolean(&items[0], scope) => true,
		Node::List(items, _, _) if matches!(items.as_slice(), [word, _] if word.drop_meta().name() == crate::lowering::tuples::RETURN) => is_boolean(&items[1], scope),
		Node::List(items, _, Separator::Semicolon | Separator::Newline) => items.last().is_some_and(|last| is_boolean(last, scope)),
		Node::List(items, _, _) => matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if BOOL_CALLS.contains(&name.as_str()) || is_bool_function(name)),
		_ => false,
	}
}

/// `x = 3 > 2`, `x: bool = …`: a variable holding a bool
pub fn is_bool_variable(name: &str, scope: &Scope) -> bool {
	scope.lookup(name).and_then(|local| local.type_node.as_deref()).is_some_and(|type_node| type_node.drop_meta().name() == BOOL_TYPE)
}
