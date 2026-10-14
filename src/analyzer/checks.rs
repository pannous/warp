//! Analysis and its diagnostics: analyze, type errors, function collection, lints and the checks of calls, nulls and declarations

use super::*;

mod type_names;
pub use type_names::*;
mod lints;
pub use lints::*;
mod declared_types;
pub use declared_types::*;

pub fn analyze(raw: Node) -> Node {
	let mut scope = Scope::new();
	if let Some(err) = check_type_errors(&raw, &mut scope) {
		return err;
	}
	raw
}

/// Check for type errors in the AST, returns Some(Node::Error) if found
pub(super) fn check_type_errors(node: &Node, scope: &mut Scope) -> Option<Node> {
	check_type_errors_inner(node, scope, false)
}

pub(super) fn check_type_errors_inner(node: &Node, scope: &mut Scope, in_structure: bool) -> Option<Node> {
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
pub(super) fn check_assignment(name: &str, value: &Node, scope: &mut Scope) -> Option<Node> {
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

pub(super) fn type_error(name: &str, existing: Kind, new: Kind) -> Node {
	Node::Error(Box::new(Node::Text(format!(
		"type mismatch: cannot assign {} to variable '{}' of type {}",
		new, name, existing
	))))
}

/// Check if two types are compatible for assignment
pub(super) fn types_compatible(existing: Kind, new: Kind) -> bool {
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

/// Kind of a built-in type name (type_tests BUILTIN_TYPES, a fixed width an Int); an exact number (`exact`, `real`) is
/// an Int that may hold a ratio (wasm_emitter/exact.rs)
pub fn builtin_type_kind(name: &str) -> Option<Kind> {
	let name = name.to_lowercase();
	crate::type_tests::held_kind(&name).or_else(|| crate::fixed_width::fixed_width(&name).map(|_| Kind::Int))
}

/// A whole-number type (`int`, `long`, `byte` …): never a fraction, unlike `exact`
pub fn is_whole_type(name: &str) -> bool {
	let name = name.trim_end_matches('?').to_lowercase();
	!crate::type_tests::is_exact_fraction_type(&name) && builtin_type_kind(&name) == Some(Kind::Int)
}

/// A declared type a decimal loses digits in: a whole type, or an inline union with a whole part and no fractional
/// one (`int or text` refuses 2.5, `int or float` takes it)
pub fn refuses_decimals(type_name: &str) -> bool {
	match union_parts(type_name.trim_end_matches('?')) {
		// a class part (`point or int`) takes no number
		Some(parts) => {
			parts.iter().any(|part| is_whole_type(part))
				&& !parts
					.iter()
					.any(|part| builtin_type_kind(part).is_some() && admits(part, Kind::Float))
		}
		None => is_whole_type(type_name),
	}
}

/// A declared type held as an exact Int, or a union of such (`rational`, `int or rational`): an IEEE float loses its
/// value there
pub fn refuses_floats(type_name: &str) -> bool {
	let type_name = type_name.trim_end_matches('?');
	let exact = |name: &str| builtin_type_kind(name) == Some(Kind::Int);
	union_parts(type_name).map_or_else(|| exact(type_name), |parts| parts.iter().all(|part| exact(part)))
}

/// Semantic checks run before emission; the first violation comes back as an error value
pub fn diagnose(program: &Node) -> Option<Node> {
	check_type_word_functions(program)
		.or_else(|| check_parameter_annotations(program))
		.or_else(|| check_variable_annotations(program))
		.or_else(|| check_union_fields(program))
		.or_else(|| {
			check_declared_types(
				program,
				&mut HashMap::new(),
				&mut Scope::new(),
				&declared_classes(program),
			)
		})
		.or_else(|| super::list_views::check_widened_list_views(program))
		.or_else(|| check_constants(program, &mut HashMap::new()))
		.or_else(|| check_null_use(program, &mut HashMap::new(), &list_variables(program)))
		.or_else(|| check_subjectless_comparison(program))
		.or_else(|| check_ambiguous_calls(program))
		.or_else(|| check_call_arity(program))
		.map(Diagnostic::into_error)
}

/// `sin(0, 5)`, `pow(2)`, `√(16, 2)`: a math function or a one-value operator given another number of values, which
/// used to drop or invent values silently
pub(super) fn check_call_arity(program: &Node) -> Option<Diagnostic> {
	let context = function_definitions(program);
	call_arity_error(program, &context)
}

pub(super) fn call_arity_error(node: &Node, context: &Context) -> Option<Diagnostic> {
	let values = |count: usize| {
		if count == 1 {
			"1 value".to_string()
		} else {
			format!("{count} values")
		}
	};
	match node {
		Node::Meta { node, .. } => call_arity_error(node, context),
		// `import (sin, floor) from 'm'` names functions, it calls none
		Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if crate::modules::is_import_keyword(word)) => {
			None
		}
		Node::List(items, bracket, _) => {
			if let (Bracket::Round, Some(Node::Symbol(name))) = (bracket, items.first().map(Node::drop_meta)) {
				let libm = crate::ffi::LIBM_F64_FUNCTIONS
					.iter()
					.filter(|(function, _)| function == name)
					.map(|(_, arity)| *arity);
				let arities: Vec<usize> = libm
					.chain(crate::wasm_emitter::text_builtins::text_builtin_arities(name))
					.collect();
				let given = items.len() - 1;
				if !arities.is_empty() && !context.user_functions.contains_key(name) && !arities.contains(&given) {
					return Some(Diagnostic::at(
						node,
						format!("{name} takes {}, got {given}", values(arities[0])),
					));
				}
			}
			items.iter().find_map(|item| call_arity_error(item, context))
		}
		Node::Key(empty, op @ (Op::Sqrt | Op::Cbrt | Op::Abs), operand) if empty.is_nothing() => {
			match operand.drop_meta() {
				Node::List(items, Bracket::Round, Separator::Colon) if items.len() > 1 => Some(Diagnostic::at(
					node,
					format!("{op} takes 1 value, got {}", operand.serialize()),
				)),
				_ => call_arity_error(operand, context),
			}
		}
		Node::Key(left, _, right) => call_arity_error(left, context).or_else(|| call_arity_error(right, context)),
		_ => None,
	}
}

/// `sqrt(x) := …`, `def abs(x): …`: a prefix operator word reads as the operator, so the definition could never be
/// called (P141); it arrives as `(√ ø x) := …`. Checked on the source, before a pass reads the operator
pub fn check_operator_word_functions(program: &Node) -> Option<Diagnostic> {
	// a method may be named so: `p.norm()` reads as the method (card class-method-named)
	let mut methods = std::collections::HashSet::new();
	program.visit(&mut |node| {
		if let Node::Type { body, .. } = node {
			body.visit(&mut |part| {
				methods.insert(part as *const Node);
			});
		}
	});
	let mut clash = None;
	program.visit(&mut |node| {
		if clash.is_some() || methods.contains(&(node as *const Node)) {
			return;
		}
		let defined = match node {
			Node::Key(head, Op::Define | Op::Assign, _) => operator_word(head),
			Node::List(items, _, _) => match (items.first(), items.get(1).map(Node::drop_meta)) {
				(Some(def), Some(Node::Key(head, Op::Colon, _)))
					if crate::operators::is_function_keyword(&crate::declarations::word(def)) =>
				{
					operator_word(head)
				}
				(Some(def), Some(head)) if crate::operators::is_function_keyword(&crate::declarations::word(def)) => {
					operator_word(head)
				}
				_ => None,
			},
			_ => None,
		};
		clash = defined.map(|(word, op)| (node.clone(), word, op));
	});
	let (definition, word, op) = clash?;
	Some(Diagnostic::at(
		&definition,
		format!("{word} is an operator ({op}); rename your function"),
	))
}

/// The operator word a definition's head names: `norm(x)` keeps its name (the parser's defines_named), also as the
/// first part of `fun norm(x) { … }`; `not(x)` arrives as the operator
fn operator_word(head: &Node) -> Option<(String, Op)> {
	match head.drop_meta() {
		Node::Key(empty, op @ (Op::Sqrt | Op::Cbrt | Op::Abs | Op::Not), _) if empty.is_nothing() => OPERATOR_WORDS
			.iter()
			.find(|(known, _)| known == op)
			.map(|(_, word)| (word.to_string(), *op)),
		Node::List(items, _, _) => match items.first().map(Node::drop_meta) {
			Some(Node::Symbol(name)) => crate::warp_parser::PREFIX_OPERATOR_WORDS
				.iter()
				.find(|(word, _)| word == name)
				.map(|(word, op)| (word.to_string(), *op)),
			Some(call @ Node::List(..)) => operator_word(call),
			_ => None,
		},
		_ => None,
	}
}

/// The words the parser reads as prefix operators (warp_parser lookahead.rs peek_prefix_operator)
const OPERATOR_WORDS: [(Op, &str); 4] = [
	(Op::Sqrt, "sqrt"),
	(Op::Cbrt, "cbrt"),
	(Op::Abs, "abs"),
	(Op::Not, "not"),
];

/// `double := it*2` or `double(x) := …`: a type word names a type, never a function (user decision P20)
pub(super) fn check_type_word_functions(program: &Node) -> Option<Diagnostic> {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, program);
	let mut clashes: Vec<&String> = ctx
		.user_functions
		.keys()
		.filter(|name| type_word_kind(name).is_some())
		.collect();
	clashes.sort();
	let name = clashes.first()?;
	let mut definition = None;
	program.visit(&mut |node| {
		if let (None, Node::Key(left, Op::Define | Op::Assign, _)) = (definition, node) {
			let head = match left.drop_meta() {
				Node::List(items, _, _) => items.first().map(Node::drop_meta),
				other => Some(other),
			};
			if matches!(head, Some(Node::Symbol(word)) if word == *name) {
				definition = Some(node);
			}
		}
	});
	Some(Diagnostic::at(
		definition.unwrap_or(program),
		TYPE_WORD_FUNCTION_CLASH.replace("{name}", name),
	))
}

/// A braceless argument takes arithmetic (`f 3-1` is `f(3-1)`, also in `1 + f 3-1`), so a second braceless call inside it
/// is ambiguous (wiki/precedence.md): `square 3 + square 3` reads as `square(3 + square 3)` or `square(3) + square(3)`.
/// Bad.md's recursive `fib it-1 + fib it-2` would silently mean `fib(it-1 + fib(it-2))`, so it is rejected with both readings.
pub(super) fn check_ambiguous_calls(node: &Node) -> Option<Diagnostic> {
	/// `return x`, `let x`, `def f`, `use m`: a statement word, no function applied braceless
	fn is_statement_word(word: &str) -> bool {
		is_declaration_word(word)
			|| crate::operators::is_function_keyword(word)
			|| crate::modules::is_import_keyword(word)
			|| RETURNING_KEYWORDS.contains(&word)
	}
	fn braceless_call(node: &Node) -> Option<(&String, &Node)> {
		match node.drop_meta() {
			Node::List(items, Bracket::None, Separator::Space) if items.len() == 2 => match items[0].drop_meta() {
				Node::Symbol(head) if !is_statement_word(head) => Some((head, &items[1])),
				_ => None,
			},
			_ => None,
		}
	}
	fn holds_call(node: &Node) -> bool {
		match node.drop_meta() {
			Node::Key(left, op, right) if op.is_arithmetic() => holds_call(left) || holds_call(right),
			other => braceless_call(other).is_some(),
		}
	}
	fn explicit(node: &Node) -> String {
		if let Some((head, argument)) = braceless_call(node) {
			return format!("{head}({})", explicit(argument));
		}
		match node.drop_meta() {
			Node::Key(left, op, right) if op.is_arithmetic() => {
				format!("{} {} {}", explicit(left), op.as_str(), explicit(right))
			}
			other => other.serialize(),
		}
	}
	if let Some((head, argument)) = braceless_call(node) {
		if let Node::Key(left, op, right) = argument.drop_meta() {
			if op.is_arithmetic() && holds_call(argument) {
				let whole = format!("{head}({})", explicit(argument));
				let first = format!("{head}({}) {} {}", explicit(left), op.as_str(), explicit(right));
				let written = format!("{head} {}", argument.drop_meta().serialize());
				return Some(
					Diagnostic::at(node, format!("ambiguous braceless call: {written}"))
						.fix(format!("{first} or {whole}"))
						.offer(format!("{head} of {} only", left.serialize()), &written, &first)
						.offer(
							format!("{head} of the whole {}", argument.drop_meta().serialize()),
							&written,
							&whole,
						),
				);
			}
		}
	}
	match node.drop_meta() {
		Node::Key(left, _, right) => check_ambiguous_calls(left).or_else(|| check_ambiguous_calls(right)),
		Node::List(items, _, _) => items.iter().find_map(check_ambiguous_calls),
		_ => None,
	}
}

/// Booleans are not numbers: `true + true` is rejected, not 2 (the runtime still encodes them as Int 1/0)
/// `> 100` outside a match arm compares nothing (card leading-gt: it was dropped silently, leaving 100)
pub(super) fn check_subjectless_comparison(node: &Node) -> Option<Diagnostic> {
	match node.drop_meta() {
		Node::Key(left, op, right) if op.is_ordering() && matches!(left.drop_meta(), Node::Empty) => {
			let written = format!("{} {}", op.as_str(), right.serialize());
			Some(Diagnostic::at(
				node,
				format!(
					"`{written}` compares nothing: write what is compared, `x {written}` (in a match arm it compares the subject)"
				),
			))
		}
		Node::Key(left, _, right) => check_subjectless_comparison(left).or_else(|| check_subjectless_comparison(right)),
		Node::List(items, _, _) => items.iter().find_map(check_subjectless_comparison),
		_ => None,
	}
}

/// Why a variable cannot be used as a plain value before an `if x {…}` check
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Unchecked {
	Null,  // `x=ø` (wiki/null.md)
	Error, // `x=fetch …`: a remote call may fail (DESIGN.md "Effects": Error as Result<T, E>)
}

impl Unchecked {
	fn of(value: &Node) -> Option<Unchecked> {
		match value.drop_meta() {
			Node::Empty => Some(Unchecked::Null),
			_ if crate::host::fetch_call(value).is_some() => Some(Unchecked::Error),
			_ => None,
		}
	}

	fn describe(self, name: &str) -> String {
		match self {
			Unchecked::Null => format!("{name} may be ø (null)"),
			Unchecked::Error => format!("{name} may be an error (fetch can fail)"),
		}
	}
}

/// A list literal `[1, 2]` or a variable assigned one
fn holds_list(operand: &Node, lists: &HashSet<String>) -> bool {
	match operand.drop_meta() {
		Node::List(_, Bracket::Square, _) => true,
		Node::Symbol(name) => lists.contains(name),
		_ => false,
	}
}

/// The variables the program assigns a list literal: what `+` concatenates
fn list_variables(program: &Node) -> HashSet<String> {
	let mut lists = HashSet::new();
	program.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = part {
			if let (Node::Symbol(name), Node::List(_, Bracket::Square, _)) = (target.drop_meta(), value.drop_meta()) {
				lists.insert(name.clone());
			}
		}
	});
	lists
}

/// `x=ø` makes x possibly null, `x=fetch …` possibly an error: arithmetic or member access on it needs an
/// `if x {…}` check first (ø and errors are falsy). A fetch used directly as an operand is never checked.
pub(super) fn check_null_use(
	node: &Node,
	nullable: &mut HashMap<String, Unchecked>,
	lists: &HashSet<String>,
) -> Option<Diagnostic> {
	let possibly_null = |operand: &Node, nullable: &HashMap<String, Unchecked>| match operand.drop_meta() {
		Node::Symbol(name) => nullable
			.get(name)
			.map(|reason| (name.clone(), reason.describe(name), None)),
		fetched if crate::host::fetch_call(fetched).is_some() => {
			let call = fetched.serialize();
			Some((
				"result".to_string(),
				format!("{call} may be an error (fetch can fail)"),
				Some(call),
			))
		}
		_ => None,
	};
	match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, value) => {
			// `out = out + xs` building a list from ø, the empty list: a concatenation, no use of a null
			let found = match value.drop_meta() {
				Node::Key(left, Op::Add, right)
					if left.drop_meta() == target.drop_meta()
						&& nullable.get(&target.name()) == Some(&Unchecked::Null) =>
				{
					check_null_use(right, nullable, lists)
				}
				_ => check_null_use(value, nullable, lists),
			};
			let target = match target.drop_meta() {
				Node::Key(name, Op::Colon, _) => &**name, // x:int?=ø
				other => other,
			};
			if let Node::Symbol(name) = target.drop_meta() {
				match Unchecked::of(value) {
					Some(reason) => nullable.insert(name.clone(), reason),
					None => nullable.remove(name),
				};
			}
			found
		}
		Node::Key(if_condition, Op::Then, then) => {
			let Node::Key(_, Op::If, condition) = if_condition.drop_meta() else {
				return check_null_use(if_condition, nullable, lists).or_else(|| check_null_use(then, nullable, lists));
			};
			let mut narrowed = nullable.clone();
			if let Node::Symbol(name) = condition.drop_meta() {
				narrowed.remove(name);
			}
			check_null_use(condition, nullable, lists).or_else(|| check_null_use(then, &mut narrowed, lists))
		}
		// ø is the empty list: appending to it or concatenating a list needs no check (`a=(); a.add(1)`), nor popping a
		// stack built in a loop (`st=[]; for t in ts { if … { st.pop() } else { st.add(t) } }`)
		Node::Key(list, Op::Dot, call)
			if (updates_list(list, call) || popped_list(list, call).is_some())
				&& nullable.get(&list.name()) != Some(&Unchecked::Error) =>
		{
			let found = check_null_use(call, nullable, lists);
			nullable.remove(&list.name()); // no longer empty
			found
		}
		// ø is the empty list: it counts 0 (`b=[]; b.count`)
		Node::Key(list, Op::Dot, property)
			if matches!(list.drop_meta(), Node::Symbol(_))
				&& is_counting_property(&property.name())
				&& nullable.get(&list.name()) != Some(&Unchecked::Error) =>
		{
			None
		}
		// `xs + ys` of a list xs and ys = []: concatenating the empty list
		Node::Key(left, Op::Add, right) if [left, right].iter().any(|side| holds_list(side, lists)) => {
			check_null_use(left, nullable, lists).or_else(|| check_null_use(right, nullable, lists))
		}
		Node::Key(left, op, right) if op.is_arithmetic() || *op == Op::Dot => {
			let operand = possibly_null(left, nullable).or_else(|| {
				if *op == Op::Dot {
					None
				} else {
					possibly_null(right, nullable)
				}
			});
			if let Some((name, reason, call)) = operand {
				let expression = node.drop_meta().serialize();
				let fix = match call {
					Some(call) => format!("{name} = {call}; if {name} {{ {} }}", expression.replace(&call, &name)),
					None => format!("if {name} {{ {expression} }}"),
				};
				return Some(Diagnostic::at(node, format!("{reason} in {expression}")).fix(fix));
			}
			check_null_use(left, nullable, lists).or_else(|| check_null_use(right, nullable, lists))
		}
		Node::Key(left, _, right) => {
			check_null_use(left, nullable, lists).or_else(|| check_null_use(right, nullable, lists))
		}
		Node::List(items, _, _) => items.iter().find_map(|item| check_null_use(item, nullable, lists)),
		_ => None,
	}
}

/// `const x=…` binds x once: a later assignment of another value, a compound assignment, an increment or an element
/// assignment is rejected (P130); the same value again only warns (P159). `let x=…` is fully immutable: any change
/// of it is an error naming `var` (user, card let-reassign)
fn let_change(node: &Node, place: &str, change: &str) -> Diagnostic {
	Diagnostic::at(node, format!("{place} is let (immutable), cannot change it: {change}"))
		.fix(format!("use var {place}"))
}

/// The method call `add(4)`, `pop()`, `remove(x)` of `names.add(4)`: one that changes the list it is called on
fn changing_list_method(call: &Node) -> bool {
	let method = match call.drop_meta() {
		Node::List(items, _, _) => items.first().map(Node::name),
		_ => call.symbol_name().map(String::from),
	};
	method.is_some_and(|method| is_list_mutating_method(&method))
}

/// Each bound name with the text of its value, and the value nodes
type Bindings<'a> = (Vec<(String, String)>, &'a [Node]);

/// The names a `let`/`const` declaration binds, each with the text of its value, and the value nodes: `x = v`, or the
/// lowered tuple pattern `let (a, b) = v, w` (`$destructure (a, b) v w`, one value unpacked when there is only one)
fn declared_bindings(declaration: &Node) -> Option<Bindings<'_>> {
	if let Node::Key(target, Op::Assign | Op::Define, value) = declaration.drop_meta() {
		return Some((vec![(target.name(), value.serialize())], std::slice::from_ref(value)));
	}
	let (names, values) = crate::tuples::destructuring(declaration)?;
	let value_of = |index: usize| match values {
		[single] => format!("{}#{}", single.serialize(), index + 1),
		_ => values[index].serialize(),
	};
	Some((
		names
			.into_iter()
			.enumerate()
			.map(|(index, name)| (name, value_of(index)))
			.collect(),
		values,
	))
}

pub(super) fn check_constants(node: &Node, constants: &mut HashMap<String, (String, String)>) -> Option<Diagnostic> {
	// a function's own `let`/`const` bind in it only; it still sees main's (card serve-var)
	if let Some(body) = super::variables::function_definition_body(node) {
		return check_constants(body, &mut constants.clone());
	}
	match node.drop_meta() {
		Node::List(items, _, _) => {
			let statements = match items.as_slice() {
				[keyword, declaration, rest @ ..]
					if is_constant_keyword(keyword) || keyword.is_symbol(IMMUTABLE_LET) =>
				{
					let keyword = keyword.name();
					let Some((bindings, values)) = declared_bindings(declaration) else {
						return Some(
							Diagnostic::at(
								declaration,
								format!("{keyword} needs a value: {}", declaration.serialize()),
							)
							.fix(format!("{keyword} x = 5")),
						);
					};
					if let Some(found) = values.iter().find_map(|value| check_constants(value, constants)) {
						return Some(found);
					}
					constants.extend(
						bindings
							.into_iter()
							.map(|(name, value)| (name, (keyword.clone(), value))),
					);
					rest
				}
				_ => items.as_slice(),
			};
			statements
				.iter()
				.find_map(|statement| check_constants(statement, constants))
		}
		Node::Key(target, op, value)
			if matches!(op, Op::Assign | Op::Define | Op::Inc | Op::Dec) || op.is_compound_assign() =>
		{
			let place = match target.drop_meta() {
				Node::Key(name, Op::Hash, _) => name.name(),
				other => other.name(),
			};
			let assignment = node.drop_meta().serialize();
			match constants.get(&place) {
				Some((keyword, _)) if keyword == IMMUTABLE_LET => return Some(let_change(node, &place, &assignment)),
				Some((_, bound))
					if *op == Op::Assign
						&& matches!(target.drop_meta(), Node::Symbol(_))
						&& value.serialize() == *bound =>
				{
					let redundant = Diagnostic::at(
						node,
						format!("{place} is const and already {bound}: {assignment} changes nothing"),
					)
					.fix("remove the redundant assignment".to_string());
					if crate::diagnostic::report(std::slice::from_ref(&redundant)).is_err() {
						return Some(redundant);
					}
				}
				Some(_) => {
					return Some(
						Diagnostic::at(node, format!("{place} is const, cannot assign it again: {assignment}")).fix(
							format!("use a new name instead of {place}, or declare it without const"),
						),
					);
				}
				None => {}
			}
			check_constants(value, constants)
		}
		// `names.add(4)`, `names.pop()` change the list as an assignment would (card const-list-add)
		Node::Key(list, Op::Dot, call) if changing_list_method(call) => {
			let place = list.name();
			let change = node.drop_meta().serialize();
			match constants.get(&place) {
				Some((keyword, _)) if keyword == IMMUTABLE_LET => return Some(let_change(node, &place, &change)),
				Some(_) => {
					return Some(
						Diagnostic::at(node, format!("{place} is const, cannot change it: {change}")).fix(format!(
							"change a copy (copy = {place}), or declare {place} without const"
						)),
					);
				}
				None => {}
			}
			check_constants(call, constants)
		}
		Node::Key(left, _, right) => check_constants(left, constants).or_else(|| check_constants(right, constants)),
		_ => None,
	}
}

/// `"3"+3`: a text spelling a number added to a number (card print-oldest: "a"+3 joins silently, "3"+3 warns); the
/// spelling text, quoted
fn number_text_plus_number(left: &Node, right: &Node) -> Option<String> {
	let spelled = |text: &Node, number: &Node| match (text.drop_meta(), number.drop_meta()) {
		(Node::Text(digits), Node::Number(_)) if digits.trim().parse::<f64>().is_ok() => {
			Some(text.serialize().trim().to_string())
		}
		(Node::Char(digit), Node::Number(_)) if digit.is_ascii_digit() => Some(format!("\"{digit}\"")),
		_ => None,
	};
	spelled(left, right).or_else(|| spelled(right, left))
}

fn number_text_plus_number_warning(node: &Node, left: &Node, right: &Node, spelled: &str) -> Diagnostic {
	let is_number = |side: &Node| matches!(side.drop_meta(), Node::Number(_));
	let written_with = |number_form: &dyn Fn(String) -> String, text_form: &dyn Fn(&str) -> String| {
		let operand = |side: &Node| {
			if is_number(side) {
				number_form(side.serialize().trim().to_string())
			} else {
				text_form(spelled)
			}
		};
		format!("{} + {}", operand(left), operand(right))
	};
	let written = written_with(&|number| number, &|text| text.to_string());
	let conversion = if spelled.contains('.') { "float" } else { "int" };
	let as_sum = written_with(&|number| number, &|text| format!("{conversion}({text})"));
	let as_text = written_with(&|number| format!("str({number})"), &|text| text.to_string());
	Diagnostic::at(node, format!("{written} joins the texts; {spelled} spells a number"))
		.fix(format!("{as_sum} or {as_text}"))
		.offer("the sum of the numbers", &written, as_sum)
		.offer("the joined text", &written, as_text)
}
