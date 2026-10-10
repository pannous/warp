//! Analysis and its diagnostics: analyze, type errors, function collection, lints and the checks of calls, nulls and declarations

use super::*;

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
		Node::Key(left, _, right) => {
			check_type_errors_inner(left, scope, in_structure).or_else(|| check_type_errors_inner(right, scope, in_structure))
		}
		Node::List(items, _, _) => items.iter().find_map(|item| check_type_errors_inner(item, scope, in_structure)),
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

/// Collect all function declarations from the AST into a FunctionRegistry
pub fn collect_functions(node: &Node) -> FunctionRegistry {
	let mut registry = FunctionRegistry::new();
	collect_functions_inner(node, &mut registry);
	registry
}

pub(super) fn collect_functions_inner(node: &Node, registry: &mut FunctionRegistry) {
	let node = node.drop_meta();
	match node {
		// Pattern: fun/fn/def/define/function name(params...) body
		Node::List(items, _, _) if items.len() >= 2 => {
			if let Node::Symbol(keyword) = items[0].drop_meta() {
				if is_function_keyword(keyword) {
					if let Some(func) = parse_function_declaration(items, keyword) {
						registry.register(func);
						return;
					}
				}
			}
			// Recurse into list items
			for item in items {
				collect_functions_inner(item, registry);
			}
		}
		Node::Key(left, _, right) => {
			collect_functions_inner(left, registry);
			collect_functions_inner(right, registry);
		}
		_ => {}
	}
}

/// Parse a function declaration from a list starting with fun/fn/def/define/function
pub(super) fn parse_function_declaration(items: &[Node], _keyword: &str) -> Option<Function> {
	// Structure: [keyword, ((name (type param)...) body)]
	// or: [keyword, ((name params...) body)]
	if items.len() < 2 {
		return None;
	}

	let decl = items[1].drop_meta();

	// Get function name and params from the declaration structure
	let (name, params, body) = match decl {
		// Pattern: ((name params...) body)
		Node::List(decl_items, _, _) if !decl_items.is_empty() => {
			let first = decl_items[0].drop_meta();
			match first {
				// (name params...)
				Node::List(name_params, _, _) if !name_params.is_empty() => {
					let func_name = name_params[0].name();
					let params = &name_params[1..];
					let body = if decl_items.len() > 1 {
						Some(Box::new(decl_items[1].clone()))
					} else {
						None
					};
					(func_name, params.to_vec(), body)
				}
				// Just a name symbol
				Node::Symbol(name) => {
					let body = if decl_items.len() > 1 {
						Some(Box::new(decl_items[1].clone()))
					} else {
						None
					};
					(name.clone(), Vec::new(), body)
				}
				_ => return None,
			}
		}
		_ => return None,
	};

	if name.is_empty() {
		return None;
	}

	let mut func = Function::new(&name);
	func.body = body;

	for param in &params {
		let (param_name, param_kind) = parse_param(param);
		func.signature.add(&param_name, param_kind);
	}

	Some(func)
}

/// Parse a parameter node into (name, kind)
pub(super) fn parse_param(param: &Node) -> (String, Kind) {
	let param = param.drop_meta();
	match param {
		// Pattern: (name:type) - single item list containing a Key
		Node::List(items, _, _) if items.len() == 1 => {
			parse_param(&items[0])
		}
		// Pattern: (type name) e.g., (float a)
		Node::List(items, _, _) if items.len() >= 2 => {
			let type_name = items[0].name();
			let param_name = items[1].name();
			let kind = type_name_to_kind(&type_name);
			(param_name, kind)
		}
		// Pattern: name:type
		Node::Key(left, Op::Colon, right) => {
			let param_name = left.name();
			let type_name = right.name();
			let kind = type_name_to_kind(&type_name);
			(param_name, kind)
		}
		// Just a name (infer type later)
		Node::Symbol(name) => (name.clone(), Kind::Int),
		_ => (String::new(), Kind::Int),
	}
}

/// Convert type name string to Kind
pub(super) fn type_name_to_kind(name: &str) -> Kind {
	builtin_type_kind(name).unwrap_or(Kind::Int)
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
		Some(parts) => parts.iter().any(|part| is_whole_type(part)) && !parts.iter().any(|part| admits(part, Kind::Float)),
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

/// A plural type word denotes a list of that type: `ints`, `numbers` → `int`, `number`
pub fn plural_element_type(word: &str) -> Option<&str> {
	let singular = word.strip_suffix('s')?;
	type_word_kind(singular).map(|_| singular)
}

/// The map type of a variable held as a Node: `m: map = {}`, or `m = {}` given entries (variables.rs type_map_by_first_entry)
pub fn held_map_type(subject: &Node, scope: &Scope) -> Option<String> {
	let local = scope.binding(subject.drop_meta().symbol_name()?)?;
	let type_name = local.type_node.as_ref()?.name();
	(local.kind == Kind::Empty && type_name.starts_with(MAP_TYPE)).then_some(type_name)
}

/// The type name `type(x)` reports for a list: a list whose elements are held as Nodes is a plain `list`
pub fn shown_list_type_name(list: &Node, scope: &Scope) -> String {
	let type_name = list_type_name(list, scope);
	if type_name == NODE_LIST_TYPE { LIST_WORD.to_string() } else { type_name }
}

/// The type name of a list: `list of int` when all items share a kind (or the variable is declared `ints`), else `list`
pub fn list_type_name(list: &Node, scope: &Scope) -> String {
	const PLAIN: &str = "list";
	match list.drop_meta() {
		Node::Symbol(name) => {
			let type_name = scope.binding(name).and_then(|local| local.type_node.as_ref()).map(|type_node| type_node.name());
			match type_name {
				Some(word) => match plural_element_type(&word) {
					Some(element) => format!("{LIST_OF_PREFIX}{element}"),
					None if word.starts_with(PLAIN) || word.starts_with(MAP_TYPE) => word,
					None => PLAIN.to_string(),
				},
				// `r = f()`: the list a function returns, its elements unknown until runtime
				None if scope.binding(name).is_none() && scope.function_kind(name) == Some(Kind::List) => NODE_LIST_TYPE.to_string(),
				// a list parameter of no declared element type: each caller may pass other elements
				None if scope.binding(name).is_some_and(|local| local.is_param) => NODE_LIST_TYPE.to_string(),
				None => PLAIN.to_string(),
			}
		}
		// Grouping: `([1 2])` has the type of `[1 2]`, as in infer_type
		Node::List(items, Bracket::Round, _) if items.len() == 1 => list_type_name(&items[0], scope),
		// a block `(out = ø; for …; out)` is worth its last value
		Node::List(items, Bracket::Round | Bracket::Curly, Separator::Semicolon | Separator::Newline) if !items.is_empty() => list_type_name(&items[items.len() - 1], scope),
		// `zero_fill(count, zero)`: a list of the zero's type
		Node::List(items, _, _) if let [call, _, zero] = items.as_slice() && call.is_symbol(ZERO_FILL_CALL) => {
			format!("{LIST_OF_PREFIX}{}", element_type_word(zero, scope))
		}
		Node::List(items, Bracket::Round, _) if let [call, _] = items.as_slice() && let Some(word) = map_word(call) => {
			use crate::library_words::{MAP_ENTRIES, MAP_KEYS};
			match word {
				MAP_KEYS => format!("{LIST_OF_PREFIX}text"), // the keys of a map are read as texts
				MAP_ENTRIES => list_type_name(&items[1], scope),
				// the values of a map of known values; of anything else (a pair `a: {b: 8}`) Nodes of any type
				_ => match list_type_name(&items[1], scope) {
					map if map.starts_with(MAP_TYPE) => map.replacen(MAP_TYPE_PREFIX, LIST_OF_PREFIX, 1),
					_ => NODE_LIST_TYPE.to_string(),
				},
			}
		}
		// `sort(xs)`, `reverse(xs)`: the same elements
		Node::List(items, Bracket::Round, Separator::None) if let [order, listed] = items.as_slice()
			&& order.symbol_name().is_some_and(|word| ORDER_WORDS.contains(&word)) => list_type_name(listed, scope),
		Node::List(items, Bracket::Round, Separator::None) if let Some(word) = items.first().and_then(Node::symbol_name) => {
			// `split(t, sep)`, `chars(t)`: texts
			if TEXT_LIST_WORDS.contains(&word) {
				format!("{LIST_OF_PREFIX}text")
			} else {
				// the result of a call `f(x)` (no list literal of `f` and `x`): its elements are held as Nodes, of any type
				NODE_LIST_TYPE.to_string()
			}
		}
		// a `{key:value …}` map, `map of int` when all its values are ints
		Node::List(entries, Bracket::Curly, _) if !entries.is_empty() && entries.iter().all(|entry| matches!(entry.drop_meta(), Node::Key(_, Op::Colon, _))) => {
			let values: Vec<&Node> = entries.iter().filter_map(|entry| match entry.drop_meta() {
				Node::Key(_, _, value) => Some(value.as_ref()),
				_ => None,
			}).collect();
			// an empty `[]` fits any type of list: `{A:[["B", 4]], B:[]}` is a `map of list of list`
			let is_empty_list = |value: &&Node| matches!(value.drop_meta(), Node::Empty) || matches!(value.drop_meta(), Node::List(items, Bracket::Square, _) if items.is_empty());
			let typed: Vec<&Node> = values.iter().copied().filter(|value| !is_empty_list(value)).collect();
			let words: Vec<String> = if typed.is_empty() { &values } else { &typed }.iter().map(|value| nested_type_word(value, scope)).collect();
			let is_list_map = typed.len() < values.len() && words.iter().all(|word| word.starts_with(PLAIN));
			match common_type_word(&words) {
				Some(word) if is_list_map || typed.len() == values.len() => format!("{MAP_TYPE_PREFIX}{word}"),
				_ => MAP_TYPE.to_string(),
			}
		}
		// `xs + ys` of two lists: items of their common type, else of the union of two builtin types (held as Nodes)
		Node::Key(left, Op::Add, right) if [left, right].iter().all(|list| infer_type(list, scope) == Kind::List) => {
			let elements = [left, right].map(|list| list_type_name(list, scope));
			let words: Option<Vec<String>> = elements.iter().map(|list| list.strip_prefix(LIST_OF_PREFIX).map(str::to_string)).collect();
			let Some(words) = words else { return PLAIN.to_string() };
			match common_type_word(&words) {
				Some(word) => format!("{LIST_OF_PREFIX}{word}"),
				None if words.iter().all(|word| is_builtin_or_union(word)) => format!("{LIST_OF_PREFIX}{}", words.join(UNION_JOINER)),
				None => NODE_LIST_TYPE.to_string(),
			}
		}
		// a value of a map: `graph["A"]` of a `map of list of int` is a `list of int`; an element of a `list of list of int`
		// a `list of int`; an element of a `list of list` (`[("a", 2)]`, tuples of mixed items) holds its items as Nodes
		Node::Key(map, Op::Hash, _) => {
			let outer = list_type_name(map, scope);
			match outer.strip_prefix(MAP_TYPE_PREFIX).or_else(|| outer.strip_prefix(LIST_OF_PREFIX)) {
				Some(PLAIN) if outer.starts_with(LIST_OF_PREFIX) => NODE_LIST_TYPE.to_string(),
				Some(value_type) if value_type.starts_with(PLAIN) => value_type.to_string(),
				_ => PLAIN.to_string(),
			}
		}
		Node::List(items, _, separator) => {
			let words: Vec<String> = items.iter().map(|item| nested_type_word(item, scope)).collect();
			let is_block = matches!(separator, Separator::Semicolon | Separator::Newline);
			match common_type_word(&words) {
				Some(word) => format!("{LIST_OF_PREFIX}{word}"),
				// `[1 "a"]`: elements of different kinds, each held as its Node
				None if !words.is_empty() && !is_block => NODE_LIST_TYPE.to_string(),
				None => PLAIN.to_string(),
			}
		}
		_ => PLAIN.to_string(),
	}
}

/// The type word of an element: an item that is a list keeps its own element type, `[(0, 1), (1, 2)]` is a
/// `list of list of int`
fn nested_type_word(item: &Node, scope: &Scope) -> String {
	match infer_type(item, scope) {
		Kind::List => list_type_name(item, scope),
		_ => element_type_word(item, scope),
	}
}

/// The type name of a map literal, `map of <value type>` when its values share one
/// The got-it topic of a `let` variable that changes (P159)
const LET_CHANGES_TOPIC: &str = "let changes";
pub(crate) const MAP_TYPE: &str = "map";
/// Library words whose list result has the elements of their list argument
pub(super) const ORDER_WORDS: [&str; 2] = ["sort", "reverse"];
/// Library words whose result is a list of texts
const TEXT_LIST_WORDS: [&str; 2] = ["split", "chars"];
pub(super) const MAP_TYPE_PREFIX: &str = "map of ";
/// A list whose elements are known only at runtime (the result of a call): each element is held as a Node
pub(super) const NODE_LIST_TYPE: &str = "list of node";
pub(super) const LIST_WORD: &str = "list";
/// Between a variable and the name of a temporary made for it (`xs·range`, `xs·item`, `m·removed`)
pub const TEMPORARY_SEPARATOR: &str = "·";

/// A name only a lowering pass makes (`word·sum·3`, `x·iterator`): the program cannot write `·` in a name, where it
/// reads as a product
pub fn is_compiler_temporary(name: &str) -> bool {
	name.contains(TEMPORARY_SEPARATOR)
}
/// Statements that name functions or modules instead of calling them
pub(super) const IMPORT_WORDS: [&str; 3] = ["import", "use", "include"];
pub(super) const LIST_OF_PREFIX: &str = "list of ";
/// The type words of a bool, held as an Int (builtin_type_kind)
const BOOL_TYPES: [&str; 2] = ["bool", "boolean"];
pub(super) const OF_WORD: &str = "of";

/// The map word a call names: `map_keys`, `map_values` or `map_entries`
pub(super) fn map_word(call: &Node) -> Option<&'static str> {
	use crate::library_words::{MAP_ENTRIES, MAP_KEYS, MAP_VALUES};
	[MAP_KEYS, MAP_VALUES, MAP_ENTRIES].into_iter().find(|word| call.is_symbol(word))
}

pub(super) const INT_WORD: &str = "int";
pub(crate) const RATIONAL_WORD: &str = "rational";
pub(super) const FLOAT_WORD: &str = "float";
pub(super) const NUMBER_WORD: &str = "number";
/// Irrational constants are `real`, although the underlying representation may still be exact or float: the type name is
/// the contract, the representation may change
pub(super) const REAL_WORD: &str = "real";
pub(super) const REAL_CONSTANTS: [&str; 2] = ["π", "pi"];

/// The type word of a number literal: whole numbers are `int` (also `2.0`), exact fractions and decimals `rational`,
/// approximations (`1.5f`, √2, complex) `float`
pub fn number_type_word(number: &Number) -> &'static str {
	match number {
		Number::Int(_) | Number::BigInt(_) => INT_WORD,
		Number::Quotient(..) | Number::BigQuotient(_) => RATIONAL_WORD,
		Number::Real(_) => REAL_WORD,
		Number::Float(value) if Number::is_exact_decimal(*value) => if value.fract() == 0.0 { INT_WORD } else { RATIONAL_WORD },
		_ => FLOAT_WORD,
	}
}

/// The type word of a number written as a literal or as a quotient of integer literals (`3/4`); `None` for any other node
pub fn literal_number_type_word(node: &Node) -> Option<&'static str> {
	match node.drop_meta() {
		Node::Number(number) => Some(number_type_word(number)),
		Node::Symbol(name) if REAL_CONSTANTS.contains(&name.as_str()) => Some(REAL_WORD),
		Node::Key(numerator, Op::Div, denominator) => match (numerator.drop_meta(), denominator.drop_meta()) {
			(Node::Number(Number::Int(numerator)), Node::Number(Number::Int(denominator))) if *denominator != 0 => {
				Some(if numerator % denominator == 0 { INT_WORD } else { RATIONAL_WORD })
			}
			_ => None,
		},
		_ => None,
	}
}

pub(super) fn element_type_word(item: &Node, scope: &Scope) -> String {
	if matches!(item.drop_meta(), Node::Empty) {
		return crate::type_tests::EMPTY_TYPE.to_string();
	}
	literal_number_type_word(item).map(str::to_string).unwrap_or_else(|| infer_type(item, scope).to_string())
}

/// The one type word all element words fit: the same word, `rational` for a mix of `int` and `rational` (int is a special
/// case of rational), `number` for any other mix of numbers, optional (`int?`) when some are ø or optional (P179);
/// `None` when the elements are not all numbers or all alike
pub(super) fn common_type_word(words: &[String]) -> Option<String> {
	let is_optional = |word: &String| word == crate::type_tests::EMPTY_TYPE || word.ends_with('?');
	if words.iter().any(is_optional) {
		let values: Vec<String> = words.iter().filter(|word| *word != crate::type_tests::EMPTY_TYPE).map(|word| word.trim_end_matches('?').to_string()).collect();
		return match values.is_empty() {
			true => Some(crate::type_tests::EMPTY_TYPE.to_string()),
			false => common_type_word(&values).map(|word| format!("{word}?")),
		};
	}
	let first = words.first()?;
	if words.iter().all(|word| word == first) {
		return Some(first.clone());
	}
	let is_number = |word: &String| [INT_WORD, RATIONAL_WORD, REAL_WORD, FLOAT_WORD, NUMBER_WORD].contains(&word.as_str());
	let is_exact = |word: &String| word == INT_WORD || word == RATIONAL_WORD;
	if !words.iter().all(is_number) {
		return None;
	}
	Some(if words.iter().all(is_exact) { RATIONAL_WORD } else { NUMBER_WORD }.to_string())
}

/// Semantic checks run before emission; the first violation comes back as an error value
pub fn diagnose(program: &Node) -> Option<Node> {
	check_type_word_functions(program)
		.or_else(|| check_parameter_annotations(program))
		.or_else(|| check_declared_types(program, &mut HashMap::new(), &mut Scope::new()))
		.or_else(|| super::list_views::check_widened_list_views(program))
		.or_else(|| check_constants(program, &mut HashMap::new()))
		.or_else(|| check_null_use(program, &mut HashMap::new()))
		.or_else(|| check_subjectless_comparison(program))
		.or_else(|| check_ambiguous_calls(program))
		.or_else(|| check_call_arity(program))
		.or_else(|| check_walked_numbers(program))
		.or_else(|| check_mixed_range_bounds(program))
		.map(Diagnostic::into_error)
}

/// `sin(0, 5)`, `pow(2)`, `√(16, 2)`: a math function or a one-value operator given another number of values, which
/// used to drop or invent values silently
pub(super) fn check_call_arity(program: &Node) -> Option<Diagnostic> {
	let mut context = Context::new();
	extract_user_functions(&mut context, program);
	call_arity_error(program, &context)
}

pub(super) fn call_arity_error(node: &Node, context: &Context) -> Option<Diagnostic> {
	let values = |count: usize| if count == 1 { "1 value".to_string() } else { format!("{count} values") };
	match node {
		Node::Meta { node, .. } => call_arity_error(node, context),
		// `import (sin, floor) from 'm'` names functions, it calls none
		Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if IMPORT_WORDS.contains(&word.as_str())) => None,
		Node::List(items, bracket, _) => {
			if let (Bracket::Round, Some(Node::Symbol(name))) = (bracket, items.first().map(Node::drop_meta)) {
				let libm = crate::ffi::LIBM_F64_FUNCTIONS.iter().filter(|(function, _)| function == name).map(|(_, arity)| *arity);
				let arities: Vec<usize> = libm.chain(crate::wasm_emitter::text_builtins::text_builtin_arities(name)).collect();
				let given = items.len() - 1;
				if !arities.is_empty() && !context.user_functions.contains_key(name) && !arities.contains(&given) {
					return Some(Diagnostic::at(node, format!("{name} takes {}, got {given}", values(arities[0]))));
				}
			}
			items.iter().find_map(|item| call_arity_error(item, context))
		}
		Node::Key(empty, op @ (Op::Sqrt | Op::Cbrt | Op::Abs), operand) if empty.is_nothing() => match operand.drop_meta() {
			Node::List(items, Bracket::Round, Separator::Colon) if items.len() > 1 => Some(Diagnostic::at(node, format!("{op} takes 1 value, got {}", operand.serialize()))),
			_ => call_arity_error(operand, context),
		},
		Node::Key(left, _, right) => call_arity_error(left, context).or_else(|| call_arity_error(right, context)),
		_ => None,
	}
}

/// `sqrt(x) := …`, `def abs(x): …`: a prefix operator word reads as the operator, so the definition could never be
/// called (P141); it arrives as `(√ ø x) := …`. Checked on the source, before a pass reads the operator
pub fn check_operator_word_functions(program: &Node) -> Option<Diagnostic> {
	// a method may be named so: `p.norm()` reads as the method (card class-method-named)
	let mut methods = std::collections::HashSet::new();
	program.visit(&mut |node| if let Node::Type { body, .. } = node {
		body.visit(&mut |part| { methods.insert(part as *const Node); });
	});
	let mut clash = None;
	program.visit(&mut |node| {
		if clash.is_some() || methods.contains(&(node as *const Node)) {
			return;
		}
		let defined = match node {
			Node::Key(head, Op::Define | Op::Assign, _) => operator_word(head),
			Node::List(items, _, _) => match (items.first(), items.get(1).map(Node::drop_meta)) {
				(Some(def), Some(Node::Key(head, Op::Colon, _))) if crate::operators::is_function_keyword(&crate::declarations::word(def)) => operator_word(head),
				(Some(def), Some(head)) if crate::operators::is_function_keyword(&crate::declarations::word(def)) => operator_word(head),
				_ => None,
			},
			_ => None,
		};
		clash = defined.map(|(word, op)| (node.clone(), word, op));
	});
	let (definition, word, op) = clash?;
	Some(Diagnostic::at(&definition, format!("{word} is an operator ({op}); rename your function")))
}

/// The operator word a definition's head names: `norm(x)` keeps its name (the parser's defines_named), also as the
/// first part of `fun norm(x) { … }`; `not(x)` arrives as the operator
fn operator_word(head: &Node) -> Option<(String, Op)> {
	match head.drop_meta() {
		Node::Key(empty, op @ (Op::Sqrt | Op::Cbrt | Op::Abs | Op::Not), _) if empty.is_nothing() => {
			OPERATOR_WORDS.iter().find(|(known, _)| known == op).map(|(_, word)| (word.to_string(), *op))
		}
		Node::List(items, _, _) => match items.first().map(Node::drop_meta) {
			Some(Node::Symbol(name)) => crate::warp_parser::PREFIX_OPERATOR_WORDS.iter().find(|(word, _)| word == name).map(|(word, op)| (word.to_string(), *op)),
			Some(call @ Node::List(..)) => operator_word(call),
			_ => None,
		},
		_ => None,
	}
}

/// The words the parser reads as prefix operators (warp_parser lookahead.rs peek_prefix_operator)
const OPERATOR_WORDS: [(Op, &str); 4] = [(Op::Sqrt, "sqrt"), (Op::Cbrt, "cbrt"), (Op::Abs, "abs"), (Op::Not, "not")];

/// `double := it*2` or `double(x) := …`: a type word names a type, never a function (user decision P20)
pub(super) fn check_type_word_functions(program: &Node) -> Option<Diagnostic> {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, program);
	let mut clashes: Vec<&String> = ctx.user_functions.keys().filter(|name| type_word_kind(name).is_some()).collect();
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
	Some(Diagnostic::at(definition.unwrap_or(program), TYPE_WORD_FUNCTION_CLASH.replace("{name}", name)))
}

/// Warnings that do not stop compilation: well-defined code that likely does not mean what it says
pub fn lint(program: &Node) -> Vec<Diagnostic> {
	let mut warnings = vec![];
	lint_into(program, &mut warnings);
	warnings.extend(kebab_ambiguities(program));
	program.visit(&mut |node| {
		if let Node::Key(_, Op::Define, body) = node {
			if uses_it_outside_loops(body) {
				hidden_function_it(body, &mut warnings);
			}
		}
	});
	warnings
}

/// The warnings of the program as written, before the passes rewrite `it` (P174 braced_it_body)
pub fn source_warnings(program: &Node) -> Vec<Diagnostic> {
	let mut warnings = vec![];
	program.visit(&mut |node| {
		if let Node::Key(head, Op::Define, body) = node {
			warnings.extend(braced_it_body(head, body));
		}
	});
	warnings
}

/// P174: `mk(k) := { it * k }`, a body that is one braced block reading `it`: it is the parameter k (mk(3) is 9), though
/// Kotlin/Swift readers see a returned lambda; both readings are offered
fn braced_it_body(head: &Node, body: &Node) -> Option<Diagnostic> {
	let Node::List(items, _, _) = head.drop_meta() else { return None };
	let [name, parameter] = items.as_slice() else { return None };
	let (Node::Symbol(name), Node::Symbol(parameter)) = (name.drop_meta(), parameter.drop_meta()) else { return None };
	if !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) || !uses_it_outside_loops(body) {
		return None;
	}
	// a one-statement block reads without its braces: `x => x*k`
	let statement = match body.drop_meta() {
		Node::List(items, _, _) if items.len() == 1 => items[0].clone(),
		other => other.clone(),
	};
	let substituted = |word: &str| crate::library_words::substitute(statement.clone(), "it", &Node::Symbol(word.to_string()));
	let written = format!("{name}({parameter}) := {}", body.serialize().trim());
	let as_parameter = format!("{name}({parameter}) := {}", substituted(parameter).serialize().trim());
	let as_lambda = format!("{name}({parameter}) := x => {}", substituted("x").serialize().trim());
	Some(Diagnostic::at(body, format!("`it` in {name}'s braced body is its parameter {parameter}, not the parameter of a returned lambda"))
		.fix(format!("{as_parameter} or {as_lambda}"))
		.offer(format!("it is the parameter {parameter}"), &written, as_parameter)
		.offer("a lambda that the function returns", &written, as_lambda))
}

/// `for 1..4 {…}`: a loop that binds the implicit `it`
/// `for 1..4 {…}` and `for i in xs {…}`: loops that bind the implicit `it` (to the item); their iterable and body
pub(super) fn it_loop(items: &[Node]) -> Option<(&Node, &Node)> {
	let (keyword, iterable, body) = match items {
		[keyword, iterable, body] => (keyword, iterable, body),
		[keyword, _, in_word, iterable, body] if in_word.is_symbol("in") => (keyword, iterable, body),
		_ => return None,
	};
	(keyword.is_symbol("for") && matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _))).then_some((iterable, body))
}

/// Does a function body read its implicit parameter `it`, not counting the `it` its loops bind?
pub(super) fn uses_it_outside_loops(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(word) => word == "it",
		Node::Key(left, _, right) => uses_it_outside_loops(left) || uses_it_outside_loops(right),
		Node::List(items, _, _) if let Some((iterable, _)) = it_loop(items) => uses_it_outside_loops(iterable),
		Node::List(items, _, _) => items.iter().any(uses_it_outside_loops),
		_ => false,
	}
}

/// The name the fix of a hidden `it` gives the function's `it`, so the loop can read it
pub(super) const FUNCTION_IT: &str = "outer_it";

/// The loops in a function with an implicit `it` whose own `it` hides the function's (user decision #23: kept, warned)
pub(super) fn hidden_function_it(body: &Node, warnings: &mut Vec<Diagnostic>) {
	body.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			if it_loop(items).is_some_and(|(_, body)| uses_it(body)) {
				let renamed = crate::library_words::substitute(node.clone(), "it", &Node::Symbol(FUNCTION_IT.to_string()));
				let explicit = format!("{FUNCTION_IT}=it; {}", renamed.serialize().trim());
				warnings.push(Diagnostic::at(node, "the loop's `it` hides the function's `it` inside the loop")
					.fix(format!("name the function's it before the loop: {FUNCTION_IT}=it; for … {{… {FUNCTION_IT} …}}"))
					.offer("the function's `it` inside the loop", node.serialize().trim(), explicit));
			}
		}
	});
}

pub(super) fn lint_into(node: &Node, warnings: &mut Vec<Diagnostic>) {
	match node.drop_meta() {
		Node::Key(left, op, right) => {
			if let (Op::Or, Node::Key(condition, Op::And, then)) = (op, left.drop_meta()) {
				let (condition, then, otherwise) = (condition.serialize(), then.serialize(), right.serialize());
				let explicit = format!("if {condition} then {then} else {otherwise}");
				warnings.push(Diagnostic::at(node, format!("`{condition} and {then} or {otherwise}` yields {otherwise} whenever {then} is falsy"))
					.fix(&explicit).offer(format!("{then} whenever {condition} holds"), format!("{condition} and {then} or {otherwise}"), explicit));
			}
			if *op == Op::As && is_arithmetic(left) {
				let diagnostic = Diagnostic::at(node, conversion_of_arithmetic_warning(left, right));
				let written = format!("{} as {}", left.serialize(), right.serialize());
				let readings = conversion_readings(left, right);
				let forms: Vec<&str> = readings.iter().map(|(_, form)| form.as_str()).collect();
				let diagnostic = diagnostic.fix(forms.join(" or "));
				warnings.push(readings.iter().fold(diagnostic, |diagnostic, (meaning, form)| diagnostic.offer(meaning, &written, form)));
			}
			if *op == Op::Mod && (is_negative(left) || is_negative(right)) {
				let (a, b) = (left.serialize(), right.serialize());
				warnings.push(Diagnostic::at(node, negative_modulo_warning(left, right))
					.offer("the truncated remainder of C/Java/JS", format!("{} % {}", a.trim(), b.trim()), format!("{} rem {}", a.trim(), b.trim())));
			}
			if let (Op::Add, Some(spelled)) = (op, number_text_plus_number(left, right)) {
				warnings.push(number_text_plus_number_warning(node, left, right, &spelled));
			}
			lint_into(left, warnings);
			lint_into(right, warnings);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| lint_into(item, warnings)),
		_ => {}
	}
}

/// `name:literal` as a data binding: the name and its value
pub(super) fn data_binding(item: &Node) -> Option<(&str, &Node)> {
	let Node::Key(key, Op::Colon, value) = item.drop_meta() else { return None };
	let Node::Symbol(name) = key.drop_meta() else { return None };
	// a value, or data made of values: `colors:{red:(1 0 0)}` is read by a sibling as `colors.red` (wiki variable.md)
	let is_data = matches!(value.drop_meta(), Node::Number(_) | Node::Text(_) | Node::Char(_))
		|| matches!(value.drop_meta(), Node::List(items, _, _) if !items.is_empty()) && is_data_node(value)
		|| matches!(value.drop_meta(), Node::Key(from, Op::Range | Op::To, to) if is_data_node(from) && is_data_node(to));
	is_data.then(|| (name.as_str(), value.drop_meta()))
}

pub(super) fn assigned_names(program: &Node) -> HashSet<&str> {
	let mut names = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Key(target, Op::Assign | Op::Define, _) = node {
			if let Node::Symbol(name) = target.drop_meta() {
				names.insert(name.as_str());
			}
		}
	});
	names
}

/// A data key `a-b:2` also reads as the subtraction `a - b` when `a` and `b` are variables
pub(super) fn kebab_ambiguities(program: &Node) -> Vec<Diagnostic> {
	let variables = assigned_names(program);
	let mut warnings = vec![];
	program.visit(&mut |item| {
		let Some((name, _)) = data_binding(item) else { return };
		let parts: Vec<&str> = name.split('-').collect();
		if parts.len() < 2 || !parts.iter().all(|part| variables.contains(part)) {
			return;
		}
		let subtraction = parts.join(" - ");
		let message = format!("`{name}` is a data key here, but {} are also variables: `{subtraction}` would subtract", parts.join(" and "));
		let warning = Diagnostic::at(item, message).fix(format!("write {subtraction} for the subtraction"));
		warnings.push(kebab_fixes(warning, program, name, &subtraction));
	});
	warnings
}

/// The readings of a kebab data key (user, P81): the data key quoted, when nothing reads it bare; the subtraction at
/// each bare read (the key quoted); the data key renamed with underscores, at the key and at each bare read
pub(super) fn kebab_fixes(warning: Diagnostic, program: &Node, name: &str, subtraction: &str) -> Diagnostic {
	let mut reads = vec![];
	bare_reads(program, name, &mut reads);
	let quoted = format!("\"{name}\"");
	let at_reads = |fix: crate::fixits::Fix, replacement: &str| reads.iter().fold(fix, |fix, read| fix.and(name, replacement, *read));
	let renamed = name.replace('-', "_");
	let warning = match reads.is_empty() {
		true => warning.offer(format!("the data key {name}"), name, quoted),
		false => warning.offering(at_reads(crate::fixits::fix(format!("the subtraction {subtraction}"), name, quoted), subtraction)),
	};
	warning.offering(at_reads(crate::fixits::fix(format!("the data key, renamed {renamed}"), name, &renamed), &renamed))
}

/// Where `name` is read bare: not as a data key (`a-b:2`) or a member (`x.a-b`), which already mean the key
pub(super) fn bare_reads(node: &Node, name: &str, reads: &mut Vec<(usize, usize)>) {
	match node {
		Node::Meta { node: inner, .. } if inner.is_symbol(name) => reads.extend(crate::diagnostic::position(node)),
		Node::Meta { node, .. } => bare_reads(node, name, reads),
		Node::Key(_, Op::Colon, value) if data_binding(node).is_some() => bare_reads(value, name, reads),
		Node::Key(left, Op::Dot, _) => bare_reads(left, name, reads),
		Node::Key(left, _, right) => {
			bare_reads(left, name, reads);
			bare_reads(right, name, reads);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| bare_reads(item, name, reads)),
		_ => {}
	}
}

/// A symbol that is not a variable reads the value of the data key of that name given earlier in the same block:
/// `a-b:2 c-d:4 a-b` is 2 (the data is the scope)
pub fn resolve_data_scope(program: Node) -> Node {
	let mut variables: HashSet<String> = assigned_names(&program).into_iter().map(String::from).collect();
	let mut context = Context::new();
	extract_user_functions(&mut context, &program);
	variables.extend(context.user_functions.values().flat_map(|function| function.params.iter().map(|param| param.name.clone())));
	let resolved = resolve_blocks(program, &variables);
	subtract_kebab_variables(resolved, &variables)
}

/// P168 (user): `p.phone-number` of an object without that field, `number` a variable, is the subtraction
/// `p.phone - number`; an object whose fields are unknown keeps the field read. Early, before member reads become indexes
pub fn lower_kebab_members(program: Node) -> Node {
	let objects = literal_object_fields(&program);
	if objects.is_empty() {
		return program;
	}
	let variables: HashSet<String> = assigned_names(&program).into_iter().map(String::from).collect();
	subtract_kebab_members(program, &variables, &objects)
}

/// The fields of the variables assigned object literals (`p = {phone: 7}`), over every such assignment
fn literal_object_fields(program: &Node) -> HashMap<String, HashSet<String>> {
	let mut objects: HashMap<String, HashSet<String>> = HashMap::new();
	program.visit(&mut |node| {
		let Node::Key(target, Op::Assign | Op::Define, value) = node else { return };
		let (Node::Symbol(name), Node::List(entries, Bracket::Curly, _)) = (target.drop_meta(), value.drop_meta()) else { return };
		let keys: Option<Vec<String>> = entries.iter().map(|entry| match entry.drop_meta() {
			Node::Key(key, Op::Colon, _) => Some(key.name()),
			_ => None,
		}).collect();
		if let Some(keys) = keys.filter(|keys| !keys.is_empty()) {
			objects.entry(name.clone()).or_default().extend(keys);
		}
	});
	objects
}

fn subtract_kebab_members(node: Node, variables: &HashSet<String>, objects: &HashMap<String, HashSet<String>>) -> Node {
	match node {
		Node::Key(receiver, Op::Dot, member) if let Some((field, rest)) = kebab_member(&receiver, &member, variables, objects) => {
			let read = Node::Key(receiver, Op::Dot, Box::new(Node::Symbol(field)));
			rest.into_iter().fold(read, |difference, term| Node::Key(Box::new(difference), Op::Sub, Box::new(Node::Symbol(term))))
		}
		other => other.map_children(|child| subtract_kebab_members(child, variables, objects)),
	}
}

/// `p.a-b-c`: the field a and the variables b, c, when p's known fields lack `a-b-c`
fn kebab_member(receiver: &Node, member: &Node, variables: &HashSet<String>, objects: &HashMap<String, HashSet<String>>) -> Option<(String, Vec<String>)> {
	let (Node::Symbol(object), Node::Symbol(name)) = (receiver.drop_meta(), member.drop_meta()) else { return None };
	let fields = objects.get(object)?;
	let (field, rest) = name.split_once('-')?;
	let rest: Vec<String> = rest.split('-').map(str::to_string).collect();
	(!fields.contains(name) && rest.iter().all(|part| variables.contains(part))).then(|| (field.to_string(), rest))
}

/// A hyphenated name that is no data key and whose parts are all variables is their difference: `a=5; b=1; a-b` is 4.
/// The names of data keys and assignment targets stay as they are.
pub(super) fn subtract_kebab_variables(node: Node, variables: &HashSet<String>) -> Node {
	match node {
		Node::Symbol(name) => {
			let parts: Vec<&str> = name.split('-').collect();
			if parts.len() > 1 && parts.iter().all(|part| variables.contains(*part)) {
				let mut terms = parts.into_iter().map(|part| Node::Symbol(part.to_string()));
				let first = terms.next().expect("split gives a part");
				terms.fold(first, |difference, term| Node::Key(Box::new(difference), Op::Sub, Box::new(term)))
			} else {
				Node::Symbol(name)
			}
		}
		Node::Key(left, op @ (Op::Colon | Op::Assign | Op::Define), right) => {
			Node::Key(left, op, Box::new(subtract_kebab_variables(*right, variables)))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(subtract_kebab_variables(*left, variables)), op, Box::new(subtract_kebab_variables(*right, variables))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| subtract_kebab_variables(item, variables)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(subtract_kebab_variables(*node, variables)), data },
		other => other,
	}
}

pub(super) fn resolve_blocks(node: Node, variables: &HashSet<String>) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let mut keys: HashMap<String, Node> = HashMap::new();
			let mut resolved = Vec::with_capacity(items.len());
			let mut ends_in_lookup = false;
			for item in items {
				let item = resolve_blocks(item, variables);
				ends_in_lookup = matches!(item.drop_meta(), Node::Symbol(name) if keys.contains_key(name));
				let item = substitute_keys(item, &keys);
				if let Some((name, value)) = data_binding(&item) {
					if !variables.contains(name) {
						keys.insert(name.to_string(), value.clone());
					}
				}
				resolved.push(item);
			}
			match resolved.pop() {
				Some(value) if ends_in_lookup => value,
				Some(last) => {
					resolved.push(last);
					Node::List(resolved, bracket, separator)
				}
				None => Node::List(resolved, bracket, separator),
			}
		}
		Node::Key(left, op, right) => Node::Key(Box::new(resolve_blocks(*left, variables)), op, Box::new(resolve_blocks(*right, variables))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(resolve_blocks(*node, variables)), data },
		other => other,
	}
}

pub(super) fn substitute_keys(node: Node, keys: &HashMap<String, Node>) -> Node {
	if keys.is_empty() {
		return node;
	}
	match node {
		Node::Symbol(name) => keys.get(&name).cloned().unwrap_or(Node::Symbol(name)),
		Node::Key(left, op @ (Op::Colon | Op::Assign | Op::Define), right) => Node::Key(left, op, Box::new(substitute_keys(*right, keys))),
		Node::Key(left, op, right) => Node::Key(Box::new(substitute_keys(*left, keys)), op, Box::new(substitute_keys(*right, keys))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| substitute_keys(item, keys)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(substitute_keys(*node, keys)), data },
		other => other,
	}
}

/// An ungrouped binary arithmetic expression: `2 * 1.5`, not `(2 * 1.5)`
pub(super) fn is_arithmetic(node: &Node) -> bool {
	matches!(node, Node::Key(_, op, _) if matches!(op, Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod | Op::Rem | Op::Pow))
}

pub(super) fn conversion_of_arithmetic_warning(left: &Node, right: &Node) -> String {
	format!("`{} as {}` converts the whole `{}`, not only its last operand", left.serialize(), right.serialize(), left.serialize())
}

/// Both readings of `a * b as T` with their explicit forms: the whole expression, or the nearest operand
pub(super) fn conversion_readings(left: &Node, right: &Node) -> Vec<(String, String)> {
	let (whole, target) = (left.serialize(), right.serialize());
	let mut readings = vec![(format!("convert the whole {whole}"), format!("({whole}) as {target}"))];
	if let Node::Key(first, op, last) = left {
		let tight = match last.drop_meta() {
			Node::Number(_) => format!("{}:{target}", last.serialize()),
			_ => format!("({} as {target})", last.serialize()),
		};
		readings.push((format!("convert only {}", last.serialize()), format!("{} {} {tight}", first.serialize(), op.as_str())));
	}
	readings
}

/// A negative literal or a negation: `-7`, `-x`, `(-7)`
pub(super) fn is_negative(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(Number::Int(n)) => *n < 0,
		Node::Number(Number::Float(f)) => *f < 0.0,
		Node::Number(Number::Quotient(numerator, _)) => *numerator < 0,
		Node::Number(Number::BigQuotient(q)) => q.is_negative(),
		Node::Number(Number::BigInt(big)) => big.sign() == num_bigint::Sign::Minus,
		Node::Key(left, Op::Neg | Op::Sub, _) => matches!(left.drop_meta(), Node::Empty),
		Node::List(items, _, _) if items.len() == 1 => is_negative(&items[0]),
		_ => false,
	}
}

/// `%` is Euclidean (0 ≤ r < |b|); C, Java, JS and Rust truncate, Python floors
pub(super) fn negative_modulo_warning(left: &Node, right: &Node) -> String {
	let (a, b) = (left.serialize(), right.serialize());
	let (a, b) = (a.trim(), b.trim());
	let values = match (left.drop_meta(), right.drop_meta()) {
		(Node::Number(Number::Int(x)), Node::Number(Number::Int(y))) => x.checked_rem_euclid(*y).zip(x.checked_rem(*y)),
		_ => None,
	};
	match values {
		Some((euclidean, truncated)) if euclidean != truncated => format!(
			"`{a} % {b}` is {euclidean}: % is Euclidean as in mathematics; C/Java/JS give {truncated}; use `rem` for the truncated remainder"
		),
		_ => format!("`{a} % {b}`: % is Euclidean as in mathematics (never negative), unlike C/Java/JS; use `rem` for the truncated remainder"),
	}
}

/// A braceless argument takes arithmetic (`f 3-1` is `f(3-1)`, also in `1 + f 3-1`), so a second braceless call inside it
/// is ambiguous (wiki/precedence.md): `square 3 + square 3` reads as `square(3 + square 3)` or `square(3) + square(3)`.
/// Bad.md's recursive `fib it-1 + fib it-2` would silently mean `fib(it-1 + fib(it-2))`, so it is rejected with both readings.
pub(super) fn check_ambiguous_calls(node: &Node) -> Option<Diagnostic> {
	/// `return x`, `let x`, `def f`, `use m`: a statement word, no function applied braceless
	fn is_statement_word(word: &str) -> bool {
		is_declaration_word(word) || crate::operators::is_function_keyword(word) || IMPORT_WORDS.contains(&word) || RETURNING_KEYWORDS.contains(&word)
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
			Node::Key(left, op, right) if op.is_arithmetic() => format!("{} {} {}", explicit(left), op.as_str(), explicit(right)),
			other => other.serialize(),
		}
	}
	if let Some((head, argument)) = braceless_call(node) {
		if let Node::Key(left, op, right) = argument.drop_meta() {
			if op.is_arithmetic() && holds_call(argument) {
				let whole = format!("{head}({})", explicit(argument));
				let first = format!("{head}({}) {} {}", explicit(left), op.as_str(), explicit(right));
				let written = format!("{head} {}", argument.drop_meta().serialize());
				return Some(Diagnostic::at(node, format!("ambiguous braceless call: {written}"))
					.fix(format!("{first} or {whole}"))
					.offer(format!("{head} of {} only", left.serialize()), &written, &first)
					.offer(format!("{head} of the whole {}", argument.drop_meta().serialize()), &written, &whole));
			}
		}
	}
	match node.drop_meta() {
		Node::Key(left, _, right) => check_ambiguous_calls(left).or_else(|| check_ambiguous_calls(right)),
		Node::List(items, _, _) => items.iter().find_map(check_ambiguous_calls),
		_ => None,
	}
}

/// `for x in 3 {…}`: a loop walks a list, a text or a range, never a number (a literal, or a variable only ever
/// assigned number literals); it failed only at run time with "not a list"
pub(super) fn check_walked_numbers(program: &Node) -> Option<Diagnostic> {
	let is_number = |value: &Node| matches!(value.drop_meta(), Node::Number(_));
	let mut numbers: HashMap<String, bool> = HashMap::new();
	program.visit(&mut |part| if let Node::Key(target, Op::Assign | Op::Define, value) = part {
		if let Node::Symbol(name) = target.drop_meta() {
			*numbers.entry(name.clone()).or_insert(true) &= is_number(value);
		}
	});
	let mut found = None;
	program.visit(&mut |part| if let (None, Node::List(items, _, _)) = (&found, part) {
		let [keyword, variable, within, walked, ..] = items.as_slice() else { return };
		let walks_number = is_number(walked) || matches!(walked.drop_meta(), Node::Symbol(name) if numbers.get(name) == Some(&true));
		if walks_number && keyword.drop_meta().name() == "for" && within.drop_meta().name() == "in" {
			let (variable, walked) = (variable.serialize(), walked.serialize());
			let message = format!("`for {variable} in {walked}` walks a number: a for loop walks a list, a text or a range, e.g. `for {variable} in 1 to {walked}`");
			found = Some(Diagnostic::at(part, message));
		}
	});
	found
}

/// `'a'..3`, `3 to "c"`: a range counts letters or numbers, never from one to the other; it failed only at run time
/// with "not a character"
pub(super) fn check_mixed_range_bounds(program: &Node) -> Option<Diagnostic> {
	let mut found = None;
	program.visit(&mut |part| if let (None, Node::Key(start, Op::Range | Op::To, end)) = (&found, part) {
		let (start_value, end_value) = (start.drop_meta(), end.drop_meta());
		if matches!((start_value, end_value), (Node::Char(_), Node::Number(_)) | (Node::Number(_), Node::Char(_))) {
			let written = part.serialize();
			found = Some(Diagnostic::at(part, format!("`{}` mixes a letter and a number: count letters ('a' to 'c') or numbers (1 to 3)", written.trim())));
		}
	});
	found
}

/// Booleans are not numbers: `true + true` is rejected, not 2 (the runtime still encodes them as Int 1/0)
/// `> 100` outside a match arm compares nothing (card leading-gt: it was dropped silently, leaving 100)
pub(super) fn check_subjectless_comparison(node: &Node) -> Option<Diagnostic> {
	match node.drop_meta() {
		Node::Key(left, op, right) if op.is_ordering() && matches!(left.drop_meta(), Node::Empty) => {
			let written = format!("{} {}", op.as_str(), right.serialize());
			Some(Diagnostic::at(node, format!("`{written}` compares nothing: write what is compared, `x {written}` (in a match arm it compares the subject)")))
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

/// `x=ø` makes x possibly null, `x=fetch …` possibly an error: arithmetic or member access on it needs an
/// `if x {…}` check first (ø and errors are falsy). A fetch used directly as an operand is never checked.
pub(super) fn check_null_use(node: &Node, nullable: &mut HashMap<String, Unchecked>) -> Option<Diagnostic> {
	let possibly_null = |operand: &Node, nullable: &HashMap<String, Unchecked>| match operand.drop_meta() {
		Node::Symbol(name) => nullable.get(name).map(|reason| (name.clone(), reason.describe(name), None)),
		fetched if crate::host::fetch_call(fetched).is_some() => {
			let call = fetched.serialize();
			Some(("result".to_string(), format!("{call} may be an error (fetch can fail)"), Some(call)))
		}
		_ => None,
	};
	match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, value) => {
			// `out = out + xs` building a list from ø, the empty list: a concatenation, no use of a null
			let found = match value.drop_meta() {
				Node::Key(left, Op::Add, right) if left.drop_meta() == target.drop_meta() && nullable.get(&target.name()) == Some(&Unchecked::Null) => {
					check_null_use(right, nullable)
				}
				_ => check_null_use(value, nullable),
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
				return check_null_use(if_condition, nullable).or_else(|| check_null_use(then, nullable));
			};
			let mut narrowed = nullable.clone();
			if let Node::Symbol(name) = condition.drop_meta() {
				narrowed.remove(name);
			}
			check_null_use(condition, nullable).or_else(|| check_null_use(then, &mut narrowed))
		}
		// ø is the empty list: appending to it or concatenating a list needs no check (`a=(); a.add(1)`), nor popping a
		// stack built in a loop (`st=[]; for t in ts { if … { st.pop() } else { st.add(t) } }`)
		Node::Key(list, Op::Dot, call) if (updates_list(list, call) || popped_list(list, call).is_some()) && nullable.get(&list.name()) != Some(&Unchecked::Error) => {
			let found = check_null_use(call, nullable);
			nullable.remove(&list.name()); // no longer empty
			found
		}
		// ø is the empty list: it counts 0 (`b=[]; b.count`)
		Node::Key(list, Op::Dot, property) if matches!(list.drop_meta(), Node::Symbol(_)) && is_counting_property(&property.name()) && nullable.get(&list.name()) != Some(&Unchecked::Error) => None,
		Node::Key(left, Op::Add, right) if [left, right].iter().any(|side| matches!(side.drop_meta(), Node::List(_, Bracket::Square, _))) => {
			check_null_use(left, nullable).or_else(|| check_null_use(right, nullable))
		}
		Node::Key(left, op, right) if op.is_arithmetic() || *op == Op::Dot => {
			let operand = possibly_null(left, nullable).or_else(|| if *op == Op::Dot { None } else { possibly_null(right, nullable) });
			if let Some((name, reason, call)) = operand {
				let expression = node.drop_meta().serialize();
				let fix = match call {
					Some(call) => format!("{name} = {call}; if {name} {{ {} }}", expression.replace(&call, &name)),
					None => format!("if {name} {{ {expression} }}"),
				};
				return Some(Diagnostic::at(node, format!("{reason} in {expression}")).fix(fix));
			}
			check_null_use(left, nullable).or_else(|| check_null_use(right, nullable))
		}
		Node::Key(left, _, right) => check_null_use(left, nullable).or_else(|| check_null_use(right, nullable)),
		Node::List(items, _, _) => items.iter().find_map(|item| check_null_use(item, nullable)),
		_ => None,
	}
}

/// `const x=…` binds x once: a later assignment of another value, a compound assignment, an increment or an element
/// assignment is rejected (P130); the same value again only warns (P159). `let x=…` may change, with a note that
/// teaches `var` for a variable that changes (P159)
/// A `let` variable may change, with a got-it note teaching `var`
fn educate_let_change(node: &Node, place: &str, change: &str) {
	crate::normalize::set_position_of(node);
	crate::diagnostic::educate_once(LET_CHANGES_TOPIC, &format!("let {place}"), &format!("var {place}"), &format!("{place} changes ({change}): var says so where it is declared"));
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
	Some((names.into_iter().enumerate().map(|(index, name)| (name, value_of(index))).collect(), values))
}

pub(super) fn check_constants(node: &Node, constants: &mut HashMap<String, (String, String)>) -> Option<Diagnostic> {
	// a function's own `let`/`const` bind in it only; it still sees main's (card serve-var)
	if let Some(body) = super::variables::function_definition_body(node) {
		return check_constants(body, &mut constants.clone());
	}
	match node.drop_meta() {
		Node::List(items, _, _) => {
			let statements = match items.as_slice() {
				[keyword, declaration, rest @ ..] if is_constant_keyword(keyword) || keyword.is_symbol(IMMUTABLE_LET) => {
					let keyword = keyword.name();
					let Some((bindings, values)) = declared_bindings(declaration) else {
						return Some(Diagnostic::at(declaration, format!("{keyword} needs a value: {}", declaration.serialize())).fix(format!("{keyword} x = 5")));
					};
					if let Some(found) = values.iter().find_map(|value| check_constants(value, constants)) {
						return Some(found);
					}
					constants.extend(bindings.into_iter().map(|(name, value)| (name, (keyword.clone(), value))));
					rest
				}
				_ => items.as_slice(),
			};
			statements.iter().find_map(|statement| check_constants(statement, constants))
		}
		Node::Key(target, op, value) if matches!(op, Op::Assign | Op::Define | Op::Inc | Op::Dec) || op.is_compound_assign() => {
			let place = match target.drop_meta() {
				Node::Key(name, Op::Hash, _) => name.name(),
				other => other.name(),
			};
			let assignment = node.drop_meta().serialize();
			match constants.get(&place) {
				Some((keyword, _)) if keyword == IMMUTABLE_LET => educate_let_change(node, &place, &assignment),
				Some((_, bound)) if *op == Op::Assign && matches!(target.drop_meta(), Node::Symbol(_)) && value.serialize() == *bound => {
					let redundant = Diagnostic::at(node, format!("{place} is const and already {bound}: {assignment} changes nothing")).fix("remove the redundant assignment".to_string());
					if crate::diagnostic::report(std::slice::from_ref(&redundant)).is_err() {
						return Some(redundant);
					}
				}
				Some(_) => {
					return Some(Diagnostic::at(node, format!("{place} is const, cannot assign it again: {assignment}"))
						.fix(format!("use a new name instead of {place}, or declare it without const")));
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
				Some((keyword, _)) if keyword == IMMUTABLE_LET => educate_let_change(node, &place, &change),
				Some(_) => {
					return Some(Diagnostic::at(node, format!("{place} is const, cannot change it: {change}"))
						.fix(format!("change a copy (copy = {place}), or declare {place} without const")));
				}
				None => {}
			}
			check_constants(call, constants)
		}
		Node::Key(left, _, right) => check_constants(left, constants).or_else(|| check_constants(right, constants)),
		_ => None,
	}
}

/// A parameter annotation must name a builtin or a user-defined type, `x:flaot` is a typo
pub(super) fn check_parameter_annotations(program: &Node) -> Option<Diagnostic> {
	let mut context = Context::new();
	extract_user_functions(&mut context, program);
	let mut user_types = crate::type_kinds::TypeRegistry::new();
	collect_all_types(&mut user_types, program);
	let traits = crate::traits::Traits::of(program);
	context.user_functions.values().flat_map(|function| &function.params).find_map(|param| {
		let annotation = param.annotation.as_ref()?;
		let type_name = annotation.name();
		let is_known_name = |name: &str| type_word_kind(name).is_some() || user_types.get_by_name(name).is_some() || traits.is_trait(name);
		let known = match type_name.strip_prefix(LIST_OF_PREFIX) {
			Some(element) => is_known_name(element) || names_list_type(element),
			None => annotated_kind(annotation).is_some() || user_types.get_by_name(type_name.trim_end_matches('?')).is_some() || traits.is_trait(&type_name),
		};
		let names = crate::type_tests::builtin_type_words().map(str::to_string).chain(user_types.types().iter().map(|type_def| type_def.name.clone()));
		let diagnostic = || Diagnostic::at(annotation, format!("unknown type {type_name} of parameter {}", param.name));
		(!known).then(|| match crate::extensions::strings::near_miss(&type_name, names) {
			Some(near) => diagnostic().offer(format!("the type {near}"), &type_name, near),
			None => diagnostic(),
		})
	})
}

/// The declared name and type of the target `x:int` or `int x`
pub(crate) fn declaring_name_and_type(target: &Node) -> Option<(String, String)> {
	let (name, type_name) = match target {
		Node::Key(name, Op::Colon, type_name) => (name.drop_meta().clone(), type_name.drop_meta().clone()),
		prefixed => number_type_prefix(prefixed)?,
	};
	match (name, type_name) {
		(Node::Symbol(name), Node::Symbol(type_name)) => Some((name, type_name)),
		// `xs: [int]` is `xs: list of int`
		(Node::Symbol(name), Node::List(items, Bracket::Square, _)) => match items.as_slice() {
			[element] => Some((name, format!("{LIST_OF_PREFIX}{}", element.drop_meta().name()))),
			_ => None,
		},
		(Node::Symbol(name), union) => Some((name, union_type_name(&union)?)),
		_ => None,
	}
}

/// An inline union type joins its parts with it: `x: int | text` and `x: int or text` declare x "int or text", the
/// anonymous form of P179's sum types (card inline-union)
const UNION_JOINER: &str = " or ";
/// The empty part of a union, `int | ø`, makes it optional: `int?`
const NONE_WORDS: [&str; 3] = ["none", "nil", "null"];

/// The parts of an inline union type name: "int or text" → int, text; None for a single type
pub(crate) fn union_parts(type_name: &str) -> Option<Vec<&str>> {
	type_name.contains(UNION_JOINER).then(|| type_name.split(UNION_JOINER).collect())
}

/// The type name of an inline union `int | text`, `(int or text)`: its parts with a part another part covers dropped
/// (`int | float` is `float`, `bool | int` is `int`) and an empty part made optional (`int | ø` is `int?`); None unless
/// the parts are words and one of them is a builtin type, so a value `ok: a or b` stays a value
pub(crate) fn union_type_name(node: &Node) -> Option<String> {
	fn collect_parts(node: &Node, parts: &mut Vec<String>) -> Option<()> {
		match node.drop_meta() {
			Node::Key(left, Op::Or, right) => {
				collect_parts(left, parts)?;
				collect_parts(right, parts)
			}
			Node::List(items, Bracket::Round, _) if items.len() == 1 => collect_parts(&items[0], parts),
			Node::Symbol(word) => {
				parts.push(word.clone());
				Some(())
			}
			Node::Empty => {
				parts.push(NONE_WORDS[0].to_string());
				Some(())
			}
			_ => None,
		}
	}
	let Node::Key(_, Op::Or, _) = strip_round(node).drop_meta() else { return None };
	let mut parts = vec![];
	collect_parts(node, &mut parts)?;
	if !parts.iter().any(|part| builtin_type_kind(part).is_some()) {
		return None;
	}
	let optional = parts.iter().any(|part| NONE_WORDS.contains(&part.as_str()));
	parts.retain(|part| !NONE_WORDS.contains(&part.as_str()));
	parts.dedup();
	let admits_part = |wide: &String, narrow: &String| !is_bool_type(wide) && builtin_type_kind(narrow).is_some_and(|kind| admits(wide, kind));
	// two parts held alike (`rational | int`) admit each other: the subtype goes
	let covers = |wide: &String, narrow: &String| {
		wide != narrow && admits_part(wide, narrow) && (!admits_part(narrow, wide) || crate::type_tests::type_matches(narrow, wide))
	};
	let kept: Vec<&String> = parts.iter().filter(|part| !parts.iter().any(|other| covers(other, part))).collect();
	let joined = kept.iter().map(|part| part.as_str()).collect::<Vec<_>>().join(UNION_JOINER);
	Some(if optional { format!("{joined}?") } else { joined })
}

/// A builtin type or an inline union of builtin types: `int`, `int or text`
fn is_builtin_or_union(type_name: &str) -> bool {
	union_parts(type_name).unwrap_or_else(|| vec![type_name]).iter().all(|part| builtin_type_kind(part).is_some())
}

fn strip_round(node: &Node) -> &Node {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => strip_round(&items[0]),
		other => other,
	}
}

/// The first part of an inline union that is no builtin type: `point` of `point or text` (unions of classes come later)
pub(crate) fn unsupported_union_part(type_name: &str) -> Option<&str> {
	union_parts(type_name.trim_end_matches('?'))?.into_iter().find(|part| builtin_type_kind(part).is_none())
}

/// `x:int=…` declares x's type; every value assigned to x later must fit it
pub(super) fn check_declared_types(node: &Node, declared: &mut HashMap<String, String>, lists: &mut Scope) -> Option<Diagnostic> {
	match node.drop_meta() {
		// `names#1 = 420`: an element of a declared list
		Node::Key(target, Op::Assign, value) if indexed_list(target).is_some_and(|name| declared.contains_key(name)) => {
			let name = indexed_list(target)?;
			let type_name = &declared[name];
			let element = list_element_type(type_name)?;
			let item = Node::List(vec![value.as_ref().clone()], Bracket::Square, Separator::Space);
			list_items_mismatch(node, name, type_name, element, &item)
		}
		Node::Key(target, Op::Assign | Op::Define, value) if super::variables::function_definition_body(node).is_some() => {
			check_declared_types(value, &mut with_parameters_declared(target, declared), &mut Scope::new())
		}
		Node::Key(target, Op::Assign | Op::Define, value) => {
			let declaration = match target.drop_meta() {
				Node::Symbol(name) => declared.get_key_value(name).map(|(name, type_name)| (name.clone(), type_name.clone())),
				declaring => declaring_name_and_type(declaring),
			};
			if let Some((name, type_name)) = declaration {
				if let Some(part) = unsupported_union_part(&type_name) {
					let message = format!("{name} is declared {type_name}: an inline union joins builtin types only so far, {part} is none");
					return Some(Diagnostic::at(node, message).fix(format!("declare {name}:any to hold any value")));
				}
				declared.insert(name.clone(), type_name.clone());
				if let Some(mismatch) = assignment_mismatch(node, &name, &type_name, value).or_else(|| evident_items_mismatch(node, &name, &type_name, value, lists)) {
					return Some(mismatch);
				}
				if list_element_type(&type_name).is_some() {
					lists.define(name, Some(Box::new(Node::Symbol(type_name))), Kind::List);
				}
			} else if let Node::Symbol(name) = target.drop_meta() {
				let (kind, type_node) = super::variables::value_binding(value, lists);
				lists.define(name.clone(), type_node, kind);
			}
			check_declared_types(value, declared, lists)
		}
		// `names.add(420)`: the method call appends before mutation lowers it to `names = names + [420]`
		Node::Key(list, Op::Dot, _) if appended_items(node).is_some() => {
			let Node::Symbol(name) = list.drop_meta() else { return None };
			let type_name = declared.get(name)?;
			let element = list_element_type(type_name)?;
			let appended = Node::List(appended_items(node)?.to_vec(), Bracket::Square, Separator::Space);
			list_items_mismatch(node, name, type_name, element, &appended)
		}
		Node::Key(left, _, right) => check_declared_types(left, declared, lists).or_else(|| check_declared_types(right, declared, lists)),
		_ if crate::tuples::destructuring(node).is_some() => destructured_mismatch(node, declared),
		Node::List(items, bracket, separator) => {
			// `names: list of text = […]` still arrives as the items `names:list`, `of`, `text=[…]`
			if let Some(declaration) = of_type_declaration(items, bracket, separator) {
				return check_declared_types(&declaration, declared, lists);
			}
			let prefixed = items.windows(2).filter_map(|pair| prefixed_declaration(&pair[0], &pair[1]));
			prefixed.chain(items.iter().cloned()).find_map(|item| check_declared_types(&item, declared, lists))
		}
		_ => None,
	}
}

/// The declared types inside a function `(f n:int m)`: its annotated parameters are declared, a plain one hides an outer
/// declaration of its name
fn with_parameters_declared(head: &Node, declared: &HashMap<String, String>) -> HashMap<String, String> {
	let mut inner = declared.clone();
	let Node::List(items, _, _) = head.drop_meta() else { return inner };
	for parameter in items.iter().skip(1).map(Node::drop_meta) {
		match (declaring_name_and_type(parameter), parameter) {
			(Some((name, type_name)), _) => inner.insert(name, type_name),
			(None, Node::Symbol(name)) => inner.remove(name),
			_ => None,
		};
	}
	inner
}

/// `a, b = 2.5, 6` and `a, b = [2.5, 6]`: each value written for a declared name must fit its type, as in `a = 2.5`
fn destructured_mismatch(node: &Node, declared: &HashMap<String, String>) -> Option<Diagnostic> {
	let (names, values) = crate::tuples::destructuring(node)?;
	let values = match values {
		[one] => match strip_round(one) {
			Node::List(items, Bracket::Square | Bracket::Round, _) => items.as_slice(),
			_ => return None,
		},
		many => many,
	};
	if values.len() != names.len() {
		return None;
	}
	names.iter().zip(values).find_map(|(name, value)| assignment_mismatch(node, name, declared.get(name)?, value))
}

/// Does a value of kind `actual` fit the builtin type `type_name` (W0 subtyping, notes/type_theory.md)? An int fits a
/// float, a decimal fits `exact`, a character fits a text (`"a"` parses as a codepoint); any other type admits all here
pub(crate) fn admits(type_name: &str, actual: Kind) -> bool {
	let type_name = type_name.trim_end_matches('?');
	if let Some(parts) = union_parts(type_name) {
		return parts.iter().any(|part| admits(part, actual));
	}
	let Some(expected) = builtin_type_kind(type_name) else { return true };
	// a type held as a Node (`real`: an exact Int or a float) admits the kinds it holds at run time
	if expected == Kind::Empty {
		return crate::type_tests::runtime_kind_mask(type_name).is_none_or(|mask| mask & 1 << actual as i64 != 0);
	}
	let exact_decimal = crate::type_tests::is_exact_fraction_type(type_name) && actual == Kind::Float;
	let one_character_text = expected == Kind::Text && actual == Kind::Codepoint;
	expected == actual || (expected == Kind::Float && actual == Kind::Int) || exact_decimal || one_character_text
}

/// The builtin type a parameter annotation names: `x: text`, not `xs: [int]`
pub(crate) fn annotated_builtin_type(annotation: &Node) -> Option<&str> {
	match annotation.drop_meta() {
		Node::Symbol(name) if is_builtin_or_union(name.trim_end_matches('?')) => Some(name),
		_ => None,
	}
}

/// The element type a list annotation names: `texts`, `[text]` and `list of text` are lists of `text`
pub(crate) fn declared_element_type(annotation: &Node) -> Option<&str> {
	match annotation.drop_meta() {
		Node::Symbol(word) => list_element_type(word),
		Node::List(items, Bracket::Square, _) if items.len() == 1 => items[0].symbol_name(),
		Node::List(items, _, Separator::Space) => match items.as_slice() {
			[list, of, element] if list.is_symbol(LIST_WORD) && of.is_symbol(OF_WORD) => element.symbol_name(),
			_ => None,
		},
		_ => None,
	}
}

/// Do the elements of a list of type `list_type` (list_type_name: `list of int`) fit the declared element type? A list
/// whose elements are known only at run time (`list of node`, a plain `list`) is checked there
pub(crate) fn elements_fit(declared_element: &str, list_type: &str) -> bool {
	let Some(element) = list_type.strip_prefix(LIST_OF_PREFIX) else { return true };
	// `list of int or text` (`[1] + ["a"]`): every part must fit
	if let Some(parts) = union_parts(element) {
		return parts.iter().all(|part| elements_fit(declared_element, &format!("{LIST_OF_PREFIX}{part}")));
	}
	// a rational or real element is a fraction an int list loses (`rational` names the exact Int representation)
	let fraction = [RATIONAL_WORD, REAL_WORD, FLOAT_WORD, NUMBER_WORD].contains(&element).then_some(Kind::Float);
	match fraction.or_else(|| builtin_type_kind(element)) {
		Some(kind) => admits(declared_element, kind),
		None => true,
	}
}

pub(super) fn assignment_mismatch(assignment: &Node, name: &str, type_name: &str, value: &Node) -> Option<Diagnostic> {
	// `a = b = 2.5`: an assignment's value is the value it assigns (card chain-unchecked)
	if let Node::Key(_, Op::Assign, assigned) = value.drop_meta() {
		return assignment_mismatch(assignment, name, type_name, assigned);
	}
	if let Some(base) = type_name.strip_suffix('?') {
		return match value.drop_meta() {
			Node::Empty => None,
			_ => assignment_mismatch(assignment, name, base, value),
		};
	}
	if matches!(value.drop_meta(), Node::Empty) && builtin_type_kind(type_name).is_some() {
		let message = format!("type mismatch: {name} is declared {type_name}, cannot assign ø");
		return Some(Diagnostic::at(assignment, message).fix(format!("declare {name}:{type_name}? to allow ø"))
			.offer("allow ø", format!("{name}:{type_name}"), format!("{name}:{type_name}?")));
	}
	if let Some(element) = list_element_type(type_name) {
		return list_items_mismatch(assignment, name, type_name, element, value);
	}
	let actual = literal_misfit(type_name, value)?;
	let value_text = value.serialize();
	let message = format!("type mismatch: {name} is declared {type_name}, cannot assign {} {value_text}", format!("{actual:?}").to_lowercase());
	let actual = format!("{actual:?}").to_lowercase();
	Some(Diagnostic::at(assignment, message).fix(format!("{name}={type_name}({value_text}) or declare {name}:{actual}"))
		.offer(format!("the {type_name} of the value"), &value_text, format!("{type_name}({value_text})"))
		.offer(format!("{name} holds {actual}"), format!("{name}:{type_name}"), format!("{name}:{actual}")))
}

/// The kind of a literal `value` that does not fit the built-in type `type_name`; None when it fits or is no literal
pub(crate) fn literal_misfit(type_name: &str, value: &Node) -> Option<Kind> {
	if is_bool_type(type_name) {
		return bool_misfit(value);
	}
	if !is_builtin_or_union(type_name) {
		return None;
	}
	// a list literal fits no scalar type: `x: int = [1 2]`
	let list_literal = matches!(value.drop_meta(), Node::List(items, Bracket::Square, _) if !items.is_empty()).then_some(Kind::List);
	let actual = list_literal.or_else(|| computed_literal_kind(value))?;
	(!admits(type_name, actual)).then_some(actual)
}

/// A bool is held as an Int, but int ≰ bool: true, false and P199's 1 and 0 (yes and no) fit it, no other int
fn bool_misfit(value: &Node) -> Option<Kind> {
	match value.drop_meta() {
		Node::True | Node::False => None,
		value if is_zero_or_one(value) => None,
		_ => computed_literal_kind(value),
	}
}

/// `bool`, `boolean`: a bool, held as an Int (builtin_type_kind)
pub(crate) fn is_bool_type(type_name: &str) -> bool {
	BOOL_TYPES.contains(&type_name.trim_end_matches('?').to_lowercase().as_str())
}

/// P199 (user): 1 and 0 are yes and no wherever a bool is expected
pub(crate) fn is_zero_or_one(value: &Node) -> bool {
	matches!(value.drop_meta(), Node::Number(Number::Int(0 | 1)))
}

/// The element type of a list type: `texts` and `list of text` hold `text`
pub(crate) fn list_element_type(type_name: &str) -> Option<&str> {
	plural_element_type(type_name).or_else(|| type_name.strip_prefix(LIST_OF_PREFIX))
}

/// The items a list value is given literally: all of `[a b]`, the appended ones of `xs + [c]` and of
/// `list_extend(p.xs, [c])` (what `p.xs.add(c)` emits)
pub(crate) fn added_items(value: &Node) -> Option<&[Node]> {
	let appended = match value.drop_meta() {
		Node::Key(_, Op::Add, appended) => appended.as_ref(),
		Node::List(items, Bracket::Round, _) if matches!(items.as_slice(), [call, _, _] if call.drop_meta().name() == crate::wasm_emitter::list_ops::LIST_EXTEND) => &items[2],
		literal => literal,
	};
	match appended.drop_meta() {
		Node::List(items, Bracket::Square, _) => Some(items),
		_ => None,
	}
}

/// `xs = ys`, `xs = xs + ys` of a declared list: the items of a list whose element type is evident (`ys = ["a"]`) must
/// fit, as the run-time check would find (lists `lists` knows the element types of, as written so far)
fn evident_items_mismatch(assignment: &Node, name: &str, type_name: &str, value: &Node, lists: &Scope) -> Option<Diagnostic> {
	let element = list_element_type(type_name)?;
	let list_type = list_type_name(value, lists);
	let items = list_type.strip_prefix(LIST_OF_PREFIX).filter(|_| !elements_fit(element, &list_type))?;
	// the value may be a temporary of the element checks (`checked·1`): its items are named, not it
	let message = format!("type mismatch: {name} is declared {type_name}, cannot hold an item that is no {element}: the assigned items are {items}");
	Some(Diagnostic::at(assignment, message).fix(format!("declare {name}:list to hold any items")))
}

/// The first literal item of a list value that does not fit `element`, with its kind: of another kind, or an int
/// outside a fixed width (`xs: [int16] = [70000]`)
pub(crate) fn misfit_item<'a>(element: &str, value: &'a Node) -> Option<(&'a Node, Kind)> {
	added_items(value)?.iter().find_map(|item| Some((item, literal_misfit(element, item).or_else(|| out_of_width(element, item))?)))
}

fn out_of_width(element: &str, item: &Node) -> Option<Kind> {
	let width = crate::fixed_width::fixed_width(element)?;
	let Node::Number(Number::Int(value)) = item.drop_meta() else { return None };
	(!(width.low..=width.high).contains(&(*value as i128))).then_some(Kind::Int)
}

/// The list variable `names` of an element `names#i`
pub(crate) fn indexed_list(target: &Node) -> Option<&str> {
	let Node::Key(list, Op::Hash, _) = target.drop_meta() else { return None };
	list.symbol_name()
}

/// The items of an append call `xs.add(a)`, `xs.push(a, b)`
pub(crate) fn appended_items(call: &Node) -> Option<&[Node]> {
	let Node::Key(_, Op::Dot, method_call) = call.drop_meta() else { return None };
	let Node::List(items, _, _) = method_call.drop_meta() else { return None };
	let (method, arguments) = items.split_first()?;
	let is_append = matches!(method.drop_meta(), Node::Symbol(word) if appends(word, arguments.len()));
	is_append.then_some(arguments)
}

/// `names: texts = [420]`, `names.add(420)`: a literal item the declared element type does not take
fn list_items_mismatch(assignment: &Node, name: &str, type_name: &str, element: &str, value: &Node) -> Option<Diagnostic> {
	let (item, actual) = misfit_item(element, value)?;
	let actual = format!("{actual:?}").to_lowercase();
	let message = format!("type mismatch: {name} is declared {type_name}, cannot hold {actual} {}", item.serialize());
	Some(Diagnostic::at(assignment, message).fix(format!("declare {name}:list to hold any items")))
}

/// `"3"+3`: a text spelling a number added to a number (card print-oldest: "a"+3 joins silently, "3"+3 warns); the
/// spelling text, quoted
fn number_text_plus_number(left: &Node, right: &Node) -> Option<String> {
	let spelled = |text: &Node, number: &Node| match (text.drop_meta(), number.drop_meta()) {
		(Node::Text(digits), Node::Number(_)) if digits.trim().parse::<f64>().is_ok() => Some(text.serialize().trim().to_string()),
		(Node::Char(digit), Node::Number(_)) if digit.is_ascii_digit() => Some(format!("\"{digit}\"")),
		_ => None,
	};
	spelled(left, right).or_else(|| spelled(right, left))
}

fn number_text_plus_number_warning(node: &Node, left: &Node, right: &Node, spelled: &str) -> Diagnostic {
	let is_number = |side: &Node| matches!(side.drop_meta(), Node::Number(_));
	let written_with = |number_form: &dyn Fn(String) -> String, text_form: &dyn Fn(&str) -> String| {
		let operand = |side: &Node| if is_number(side) { number_form(side.serialize().trim().to_string()) } else { text_form(spelled) };
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
