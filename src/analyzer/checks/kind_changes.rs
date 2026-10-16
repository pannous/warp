//! Kind changes: an assignment must keep the kind of its variable
use super::*;
use crate::words::GLOBAL_WORD;

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
		Node::List(items, _, _) => items.iter().find_map(|item| check_type_errors_inner(item, scope, in_structure)),
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

/// The kind a value evidently has as written: a literal, a list literal, arithmetic of numbers, or a name or call whose
/// kind `known` gives; None for anything else (a variable, a loop variable), whose kind only inference knows
fn evident_kind(value: &Node, known: &HashMap<String, Kind>) -> Option<Kind> {
	match value.drop_meta() {
		Node::Number(Number::Float(_)) => Some(Kind::Float),
		Node::Number(_) => Some(Kind::Int),
		Node::Text(_) | Node::Char(_) => Some(Kind::Text),
		Node::List(_, Bracket::Square, _) => Some(Kind::List),
		// `3.7 as int`: the type's kind (card kind-name)
		Node::Key(_, Op::As, type_name) => super::checks::builtin_type_kind(&type_name.drop_meta().name()),
		Node::Key(left, op, right) if op.is_arithmetic() => {
			match (evident_kind(left, known)?, evident_kind(right, known)?) {
				(Kind::Int, Kind::Int) => Some(Kind::Int),
				(Kind::Int | Kind::Float, Kind::Int | Kind::Float) => Some(Kind::Float),
				_ => None,
			}
		}
		Node::Symbol(name) => known.get(name).copied(),
		Node::List(items, Bracket::Round, Separator::None) => match items.first().map(Node::drop_meta) {
			Some(Node::Symbol(function)) => known.get(&format!("{function}{CALL_SUFFIX}")).copied(),
			_ => None,
		},
		_ => None,
	}
}

/// The result kind of each user function whose last expression's kind is evident, its declared parameters known:
/// `f(x: int) := x + 1` gives an Int, `f() := "a"` a Text
fn evident_results(program: &Node) -> HashMap<String, Kind> {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, program);
	ctx.user_functions
		.values()
		.filter_map(|function| {
			let parameters = function
				.params
				.iter()
				.filter_map(|param| {
					let kind = builtin_type_kind(annotated_builtin_type(param.annotation.as_ref()?)?)?;
					Some((param.name.clone(), if kind == Kind::Codepoint { Kind::Text } else { kind }))
				})
				.collect();
			let kind = evident_kind(last_expression(&function.body), &parameters)?;
			Some((format!("{}{CALL_SUFFIX}", function.name), kind))
		})
		.collect()
}

/// The expression a body gives: its last statement
fn last_expression(body: &Node) -> &Node {
	match body.drop_meta() {
		Node::List(items, Bracket::Curly | Bracket::None, Separator::Semicolon | Separator::Newline)
			if !items.is_empty() =>
		{
			last_expression(items.last().expect("not empty"))
		}
		Node::List(items, Bracket::Curly, _) if items.len() == 1 => last_expression(&items[0]),
		other => other,
	}
}

/// P45 (user: "compile error unless we are in script mode, which is not defined yet"): a variable given values of
/// evidently different kinds that do not mix, `x = [1]; x = 5`, in the program or in one function body
pub fn check_kind_changes(program: &Node) -> Option<Diagnostic> {
	let mut bodies = vec![program];
	program.visit(&mut |node| {
		if let Node::Key(_, Op::Define, body) = node {
			bodies.push(body);
		}
	});
	let results = evident_results(program);
	let globals = global_variables(program);
	bodies.into_iter().find_map(|body| first_kind_change(body, &mut HashMap::new(), &results, &globals))
}

/// The variables declared `global y` (at this stage late_binding has moved a function's `global y` to main)
fn global_variables(program: &Node) -> HashSet<String> {
	let mut globals = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Key(keyword, Op::Colon, declared) = node.drop_meta() {
			if keyword.drop_meta().name() == GLOBAL_WORD {
				let target =
					if let Node::Key(target, Op::Assign, _) = declared.drop_meta() { target } else { declared };
				globals.insert(target.name());
			}
		}
	});
	globals
}

/// Walks the assignments in program order, not into function definitions or lambdas (their own scopes);
/// `kinds` holds each variable's evident kind, None once it was given a value of no evident kind
fn first_kind_change(
	node: &Node,
	kinds: &mut HashMap<String, Option<EvidentKind>>,
	results: &HashMap<String, Kind>,
	globals: &HashSet<String>,
) -> Option<Diagnostic> {
	match node.drop_meta() {
		Node::Key(head, Op::Define, value) => match head.drop_meta() {
			// `(f x) := …`: f names a function from here on; its body gives the variables it shares, the globals and
			// its `nonlocal` ones, values of their kinds (card nonlocal-assign)
			Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => {
				given_kind(&items[0].name(), Some(EvidentKind::Function), value, kinds).or_else(|| {
					let nonlocals = crate::late_binding::declared_nonlocals(value);
					let is_shared = |name: &String| globals.contains(name) || nonlocals.contains(name);
					let mut body_kinds: HashMap<_, _> = kinds
						.iter()
						.filter(|(name, _)| is_shared(name))
						.map(|(name, kind)| (name.clone(), *kind))
						.collect();
					let change = first_kind_change(value, &mut body_kinds, results, globals);
					kinds.extend(body_kinds.into_iter().filter(|(name, _)| is_shared(name)));
					change
				})
			}
			_ => None,
		},
		Node::Key(_, Op::Arrow, _) => None,
		Node::Key(target, Op::Assign, value) => {
			if let Some(change) = first_kind_change(value, kinds, results, globals) {
				return Some(change);
			}
			let Node::Symbol(name) = target.drop_meta() else {
				return None;
			};
			given_kind(name, EvidentKind::of(value, results), value, kinds)
		}
		Node::Key(left, _, right) => first_kind_change(left, kinds, results, globals)
			.or_else(|| first_kind_change(right, kinds, results, globals)),
		Node::List(items, _, _) => items.iter().find_map(|item| first_kind_change(item, kinds, results, globals)),
		_ => None,
	}
}

/// Marks a function's result in the `known` kinds of evident_kind, apart from a variable of the same name
const CALL_SUFFIX: &str = "()";

/// The evident kind of a value for P45's kind changes, with bools apart from ints: int ≰ bool, bool ≤ int (card bool-assign)
#[derive(Clone, Copy, PartialEq)]
enum EvidentKind {
	Of(Kind),
	Bool,
	/// a name a function definition binds, `f = x => x*2` lowered to `(f x) := x*2` included
	Function,
}

impl EvidentKind {
	fn of(value: &Node, results: &HashMap<String, Kind>) -> Option<EvidentKind> {
		// `function upper`, `&add`: the mark sits on the meta that drop_meta removes
		if crate::closures::referenced_function(value).is_some() {
			return Some(EvidentKind::Function);
		}
		match value.drop_meta() {
			Node::True | Node::False => Some(EvidentKind::Bool),
			Node::Key(_, op, _) if op.is_comparison() => Some(EvidentKind::Bool),
			Node::Key(_, Op::As, type_name) if super::checks::is_bool_type(&type_name.drop_meta().name()) => {
				Some(EvidentKind::Bool)
			}
			Node::Key(_, Op::Arrow, _) => Some(EvidentKind::Function),
			_ => evident_kind(value, results).map(EvidentKind::Of),
		}
	}

	/// The kind a variable that was `self` has after it is given `now`, None when `now` does not mix with it
	fn given(self, now: EvidentKind) -> Option<EvidentKind> {
		use EvidentKind::{Bool, Function, Of};
		match (self, now) {
			(Bool, Bool) => Some(Bool),
			(Function, Function) => Some(Function),
			(Of(Kind::Int | Kind::Float), Bool) => Some(self),
			(Of(was), Of(now)) if was == now => Some(self),
			(Of(Kind::Int | Kind::Float), Of(Kind::Int | Kind::Float)) => {
				Some(if now == Of(Kind::Float) { now } else { self })
			}
			_ => None,
		}
	}

	fn with_article(self) -> String {
		match self {
			EvidentKind::Bool => "a Bool".to_string(),
			EvidentKind::Function => "a function".to_string(),
			EvidentKind::Of(kind) => kind_with_article(kind),
		}
	}
}

/// The variable `name` is given `value` of the evident kind `given`: the kinds it holds from here on, or the change
fn given_kind(
	name: &str,
	given: Option<EvidentKind>,
	value: &Node,
	kinds: &mut HashMap<String, Option<EvidentKind>>,
) -> Option<Diagnostic> {
	// a first evident value fixes the kind (Int widens to Float); any value of no evident kind ends the check
	let kind = match (kinds.get(name).copied(), given) {
		(None, given) => given,
		// P199: 1 and 0 are yes and no, a bool variable stays one
		(Some(Some(EvidentKind::Bool)), Some(_)) if crate::analyzer::is_zero_or_one(value) => Some(EvidentKind::Bool),
		(Some(Some(was)), Some(now)) => match was.given(now) {
			Some(kind) => Some(kind),
			None => {
				let message =
					format!("{name} was {}, is given {}: use another name", was.with_article(), now.with_article());
				return Some(Diagnostic::at(value, message));
			}
		},
		_ => None,
	};
	kinds.insert(name.to_string(), kind);
	None
}
