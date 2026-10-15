//! Card bool-type (user, 2026-10-07): true and false are a shallow type of their own. They act as the Ints 1 and 0
//! everywhere, but `type(true)` is bool and they print as true/false. Which expressions are bools, statically.

use super::*;
use crate::words::IS_ERROR_WORD;

pub const BOOL_TYPE: &str = "bool";
/// Calls whose value is a bool
const BOOL_CALLS: [&str; 8] = [
	crate::library_words::COLLECTION_CONTAINS,
	crate::words::IS_TYPE,
	crate::words::INSTANCE_OF,
	IS_ERROR_WORD,
	crate::library_words::VALUES_SIMILAR,
	crate::library_words::VALUES_ROUGH,
	crate::numbers::uncertain::CERTAINLY,
	crate::numbers::uncertain::POSSIBLY,
];

/// `even(n) := n % 2 == 0`: the functions whose body ends in a bool, also through each other (`odd(n) := not even(n)`)
/// and themselves (`f(n) := if n == 0 then true else f(n - 1)`), for the emission of one program: every function is
/// taken for one until its body shows otherwise
pub fn note_bool_functions<'a>(functions: impl Iterator<Item = &'a crate::compiler::context::UserFunctionDef> + Clone) {
	crate::compiler::compile_context::change(|context| {
		context.bool_functions = functions.clone().map(|function| function.name.clone()).collect()
	});
	loop {
		let refuted: Vec<String> = functions
			.clone()
			.filter(|function| is_bool_function(&function.name) && !gives_bool(function))
			.map(|function| function.name.clone())
			.collect();
		if refuted.is_empty() {
			return;
		}
		crate::compiler::compile_context::change(|context| {
			context.bool_functions.retain(|name| !refuted.contains(name))
		});
	}
}

/// The body ends in a bool and every `return` in it returns one
pub fn gives_bool(function: &crate::compiler::context::UserFunctionDef) -> bool {
	let mut scope = parameter_scope(function);
	define_bool_locals(function, &mut scope);
	let mut returns_bool = true;
	function.body.visit(&mut |node| {
		if let Node::List(items, _, _) = node.drop_meta() {
			if let [word, value] = items.as_slice() {
				returns_bool &= word.drop_meta().name() != crate::words::RETURN_WORD || is_boolean(value, &scope);
			}
		}
	});
	returns_bool && is_boolean(&function.body, &scope)
}

/// The function's parameters declared bool (`f(b: bool) := b`), the rest unknown
fn parameter_scope(function: &crate::compiler::context::UserFunctionDef) -> Scope {
	let mut scope = Scope::new();
	for param in function
		.params
		.iter()
		.filter(|param| param.annotation.as_ref().is_some_and(|annotation| annotation.drop_meta().name() == BOOL_TYPE))
	{
		scope.define(param.name.clone(), param.annotation.clone().map(Box::new), Kind::Int);
	}
	scope
}

/// The locals of the body that only ever store bools (`hit = no; … hit = yes`, the flag `any` inlines to), also
/// through each other (`b = a`); a parameter or a local updated in place (`n += 1`) is none
fn define_bool_locals(function: &crate::compiler::context::UserFunctionDef, scope: &mut Scope) {
	let mut stored: HashMap<String, Vec<Node>> = HashMap::new();
	let mut updated: HashSet<String> = function.params.iter().map(|param| param.name.clone()).collect();
	function.body.visit(&mut |node| {
		let Node::Key(left, op, value) = node.drop_meta() else {
			return;
		};
		let Node::Symbol(name) = left.drop_meta() else {
			return;
		};
		match op {
			Op::Assign | Op::Define => stored.entry(name.clone()).or_default().push(value.as_ref().clone()),
			_ if op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec) => {
				updated.insert(name.clone());
			}
			_ => {}
		}
	});
	stored.retain(|name, _| !updated.contains(name));
	while let Some(name) = stored
		.iter()
		.find(|(_, values)| values.iter().all(|value| is_boolean(value, scope)))
		.map(|(name, _)| name.clone())
	{
		stored.remove(&name);
		scope.define(name, Some(Box::new(Node::Symbol(BOOL_TYPE.to_string()))), Kind::Int);
	}
}

pub fn is_bool_function(name: &str) -> bool {
	crate::compiler::compile_context::read(|context| context.bool_functions.contains(name))
}

/// `true`, `3 > 2`, `2 as bool`, `bool(2)`, `not x`, `a and b` of bools, `x is int`, a variable bound to one, `(…; x < 3)` ending in one
pub fn is_boolean(node: &Node, scope: &Scope) -> bool {
	match node.drop_meta() {
		Node::True | Node::False => true,
		Node::Key(left, op, right) => match op {
			_ if op.is_comparison() => true,
			Op::Not => left.is_empty_node(),
			// `2 as bool`, also a function's declared `-> bool` result (card bool-conversion)
			Op::As => is_bool_type(&right.drop_meta().name()),
			_ if op.is_logical() || op.is_bitwise() => is_boolean(left, scope) && is_boolean(right, scope),
			// `c ? yes : f(n)`, `if c then yes else f(n)`: both branches
			Op::Question => {
				matches!(right.drop_meta(), Node::Key(then, Op::Colon, otherwise) if is_boolean(then, scope) && is_boolean(otherwise, scope))
			}
			Op::Else => {
				matches!(left.drop_meta(), Node::Key(_, Op::Then, then) if is_boolean(then, scope))
					&& is_boolean(right, scope)
			}
			// `x: bool = 1` (P199): the value stored, a bool
			Op::Assign | Op::Define => {
				matches!(left.drop_meta(), Node::Symbol(name) if is_bool_variable(name, scope) || is_boolean(right, scope))
			}
			_ => false,
		},
		Node::Symbol(name) => is_bool_variable(name, scope),
		// `(x < 3)`, a braced body `def ok(x){x < 10}`
		Node::List(items, Bracket::Round | Bracket::Curly, _) if items.len() == 1 && is_boolean(&items[0], scope) => {
			true
		}
		Node::List(items, _, _) if matches!(items.as_slice(), [word, _] if word.drop_meta().name() == crate::words::RETURN_WORD) => {
			is_boolean(&items[1], scope)
		}
		Node::List(items, _, Separator::Semicolon | Separator::Newline) => {
			items.last().is_some_and(|last| is_boolean(last, scope))
		}
		Node::List(items, _, _) => {
			matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if BOOL_CALLS.contains(&name.as_str()) || is_bool_function(name) || (items.len() == 2 && is_bool_type(name)))
		}
		_ => false,
	}
}

/// `x = 3 > 2`, `x: bool = …`: a variable holding a bool
pub fn is_bool_variable(name: &str, scope: &Scope) -> bool {
	scope
		.lookup(name)
		.and_then(|local| local.type_node.as_deref())
		.is_some_and(|type_node| type_node.drop_meta().name() == BOOL_TYPE)
}
