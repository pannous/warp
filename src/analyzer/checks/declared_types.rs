//! Declared types: annotations must name known types (inline unions too), and every value given to a declared variable, parameter or list must fit
use super::*;
use crate::words::{ANY_TYPE, LIST_WORD, NUMBER_WORD, OF_WORD};

/// A parameter annotation must name a builtin or a user-defined type, `x:flaot` is a typo
pub(in crate::analyzer) fn check_parameter_annotations(program: &Node) -> Option<Diagnostic> {
	let known_types = KnownTypes::of(program);
	function_definitions(program).user_functions.values().flat_map(|function| &function.params).find_map(|param| {
		let annotation = param.annotation.as_ref()?;
		known_types.unknown_annotation(annotation, &format!("parameter {}", param.name), &param.name)
	})
}

/// A variable's declared type must be known too: `p: foo = 2.5` names no type (card unknown-type)
pub(in crate::analyzer) fn check_variable_annotations(program: &Node) -> Option<Diagnostic> {
	let known_types = KnownTypes::of(program);
	let mut unknown = None;
	program.visit(&mut |node| {
		if unknown.is_some() {
			return;
		}
		if let Node::Key(target, Op::Assign, _) = node.drop_meta() {
			if let Node::Key(name, Op::Colon, annotation) = target.drop_meta() {
				if let Some(name) = name.symbol_name() {
					unknown = known_types.unknown_annotation(annotation, name, name);
				}
			}
		}
	});
	unknown
}

/// The types a program can name: the builtin ones, its classes and its traits
pub(super) struct KnownTypes {
	pub(super) user_types: crate::type_kinds::TypeRegistry,
	pub(super) traits: crate::traits::Traits,
}

impl KnownTypes {
	pub(super) fn of(program: &Node) -> Self {
		let mut user_types = crate::type_kinds::TypeRegistry::new();
		collect_all_types(&mut user_types, program);
		KnownTypes { user_types, traits: crate::traits::Traits::of(program) }
	}

	pub(super) fn is_known_name(&self, name: &str) -> bool {
		type_word_kind(name).is_some()
			|| crate::type_tests::builtin_type_name(name).is_some()
			|| name == ANY_TYPE
			|| crate::lowering::declarations::is_collection_type_word(name)
			|| self.user_types.get_by_name(name).is_some()
			|| self.traits.is_trait(name)
	}

	/// The error for an annotation naming no known type, with the near type as its fix
	pub(super) fn unknown_annotation(&self, annotation: &Node, subject: &str, name: &str) -> Option<Diagnostic> {
		let type_name = annotation.name();
		if let Some(part) = union_parts(type_name.trim_end_matches('?'))
			.and_then(|parts| parts.into_iter().find(|part| !self.is_known_name(part)))
		{
			return Some(unknown_union_part(annotation, subject, name, &type_name, part));
		}
		let known = match type_name.strip_prefix(LIST_OF_PREFIX) {
			Some(element) => self.is_known_name(element) || names_list_type(element),
			None => annotated_kind(annotation).is_some() || self.is_known_name(type_name.trim_end_matches('?')),
		};
		let names = crate::type_tests::builtin_type_words()
			.map(str::to_string)
			.chain(self.user_types.types().iter().map(|type_def| type_def.name.clone()));
		let diagnostic = || Diagnostic::at(annotation, format!("unknown type {type_name} of {subject}"));
		(!known).then(|| match crate::extensions::strings::near_miss(&type_name, names) {
			Some(near) => diagnostic().offer(format!("the type {near}"), &type_name, near),
			None => diagnostic(),
		})
	}
}

/// The declared name and type of the target `x:int` or `int x`
pub(crate) fn declaring_name_and_type(target: &Node) -> Option<(String, String)> {
	let (name, type_name) = match target {
		Node::Key(name, Op::Colon, type_name) => (name.drop_meta().clone(), type_name.drop_meta().clone()),
		prefixed => number_type_prefix(prefixed)?,
	};
	match (name, type_name) {
		(Node::Symbol(name), Node::Symbol(type_name)) => Some((name, type_name)),
		// `xs: [int]` is `xs: list of int`, `xs: [int or error]` a list of `int or error`, no union itself
		(Node::Symbol(name), Node::List(items, Bracket::Square, _)) => match items.as_slice() {
			[element] => match union_type_name(element.drop_meta()) {
				Some(union) => Some((name, format!("{LIST_OF_PREFIX}{union}"))),
				None => Some((name, format!("{LIST_OF_PREFIX}{}", element.drop_meta().name()))),
			},
			_ => None,
		},
		(Node::Symbol(name), union) => Some((name, union_type_name(&union)?)),
		_ => None,
	}
}

/// An inline union type joins its parts with it: `x: int | text` and `x: int or text` declare x "int or text", the
/// anonymous form of P179's sum types (card inline-union)
pub(crate) const UNION_JOINER: &str = " or ";
/// The empty part of a union, `int | ø`, makes it optional: `int?`
pub(super) const NONE_WORDS: [&str; 3] = ["none", "nil", "null"];

/// The parts of an inline union type name: "int or text" → int, text; None for a single type and a list of a union
/// ("list of int or text")
pub(crate) fn union_parts(type_name: &str) -> Option<Vec<&str>> {
	(type_name.contains(UNION_JOINER) && !type_name.starts_with(LIST_OF_PREFIX))
		.then(|| type_name.split(UNION_JOINER).collect())
}

/// The type name of an inline union `int | text`, `(int or text)`: its parts with a part another part covers dropped
/// (`int | float` is `float`, `bool | int` is `int`) and an empty part made optional (`int | ø` is `int?`); None unless
/// the parts are words and one of them is a builtin type, so a value `ok: a or b` stays a value
pub(crate) fn union_type_name(node: &Node) -> Option<String> {
	union_of_types(node, &|_| false)
}

/// The type name of an inline union whose parts may also be declared classes: `Circle | Square` is `Circle or Square`
pub(crate) fn union_of_types(node: &Node, is_declared: &dyn Fn(&str) -> bool) -> Option<String> {
	pub(super) fn collect_parts(node: &Node, parts: &mut Vec<String>) -> Option<()> {
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
	let Node::Key(_, Op::Or, _) = strip_round(node).drop_meta() else {
		return None;
	};
	let mut parts = vec![];
	collect_parts(node, &mut parts)?;
	if !parts.iter().any(|part| builtin_type_kind(part).is_some() || is_declared(part)) {
		return None;
	}
	let optional = parts.iter().any(|part| NONE_WORDS.contains(&part.as_str()));
	parts.retain(|part| !NONE_WORDS.contains(&part.as_str()));
	parts.dedup();
	// only a builtin type covers another: `Point | text` keeps both, `int | zork` keeps zork to be refused
	let admits_part = |wide: &String, narrow: &String| {
		!is_bool_type(wide)
			&& builtin_type_kind(wide).is_some()
			&& builtin_type_kind(narrow).is_some_and(|kind| admits(wide, kind))
	};
	// two parts held alike (`rational | int`) admit each other: the subtype goes
	let covers = |wide: &String, narrow: &String| {
		wide != narrow
			&& admits_part(wide, narrow)
			&& (!admits_part(narrow, wide) || crate::type_tests::type_matches(narrow, wide))
	};
	let kept: Vec<&String> = parts.iter().filter(|part| !parts.iter().any(|other| covers(other, part))).collect();
	let joined = kept.iter().map(|part| part.as_str()).collect::<Vec<_>>().join(UNION_JOINER);
	Some(if optional { format!("{joined}?") } else { joined })
}

/// A builtin type or an inline union of builtin types: `int`, `int or text`
pub(super) fn is_builtin_or_union(type_name: &str) -> bool {
	union_parts(type_name).unwrap_or_else(|| vec![type_name]).iter().all(|part| builtin_type_kind(part).is_some())
}

pub(super) fn strip_round(node: &Node) -> &Node {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => strip_round(&items[0]),
		other => other,
	}
}

/// The local a checked result passes through (type_name_matching, `-> int`, `-> int or text`): errors read "the
/// result is declared int, …"
pub const CHECKED_RESULT: &str = "the result";

/// `x: int | zork`: the refusal of a union part that names no type, at the variable, parameter or field declared so
pub(super) fn unknown_union_part(at: &Node, place: &str, name: &str, type_name: &str, part: &str) -> Diagnostic {
	let message = format!(
		"{place} is declared {type_name}: an inline union joins builtin types and declared classes, {part} is neither"
	);
	Diagnostic::at(at, message).fix(format!("declare {name}:any to hold any value"))
}

/// A field `size: int | zork` of a class: its union names no type zork
pub(in crate::analyzer) fn check_union_fields(program: &Node) -> Option<Diagnostic> {
	let classes = declared_classes(program);
	let mut class_nodes = vec![];
	program.visit(&mut |part| {
		if let Node::Type { .. } = part {
			class_nodes.push(part.clone());
		}
	});
	class_nodes.iter().find_map(|class| {
		let mut registry = crate::type_kinds::TypeRegistry::new();
		registry.register_from_node(class)?;
		let type_def = registry.types().first()?;
		type_def.fields.iter().find_map(|field| {
			let part = unsupported_union_part(&field.type_name, &classes)?;
			Some(unknown_union_part(
				class,
				&format!("field {}.{}", type_def.name, field.name),
				&field.name,
				&field.type_name,
				part,
			))
		})
	})
}

/// The first part of an inline union that is neither a builtin type nor a declared class: `point` of `point or text`
pub(crate) fn unsupported_union_part<'a>(type_name: &'a str, classes: &HashSet<String>) -> Option<&'a str> {
	union_parts(type_name.trim_end_matches('?'))?
		.into_iter()
		.find(|part| !is_builtin_type(part) && !classes.contains(*part))
}

/// A type held by a kind (`int16`, `String`) or a builtin type word held as a Node (`symbol`, `error`, `time`)
fn is_builtin_type(word: &str) -> bool {
	builtin_type_kind(word).is_some() || crate::type_tests::builtin_type_name(word).is_some()
}

/// The names of the classes a program declares
pub(crate) fn declared_classes(program: &Node) -> HashSet<String> {
	let mut classes = HashSet::new();
	program.visit(&mut |part| {
		if let Node::Type { name, .. } = part {
			classes.insert(name.drop_meta().name());
		}
	});
	classes
}

/// `x:int=…` declares x's type; every value assigned to x later must fit it
pub(in crate::analyzer) fn check_declared_types(
	node: &Node,
	declared: &mut HashMap<String, String>,
	lists: &mut Scope,
	classes: &HashSet<String>,
) -> Option<Diagnostic> {
	match node.drop_meta() {
		// `names#1 = 420`: an element of a declared list
		Node::Key(target, Op::Assign, value)
			if indexed_list(target).is_some_and(|name| declared.contains_key(name)) =>
		{
			let name = indexed_list(target)?;
			let type_name = &declared[name];
			let element = list_element_type(type_name)?;
			let item = Node::List(vec![value.as_ref().clone()], Bracket::Square, Separator::Space);
			list_items_mismatch(node, name, type_name, element, &item)
		}
		Node::Key(target, Op::Assign | Op::Define, value)
			if super::super::variables::function_definition_body(node).is_some() =>
		{
			check_declared_types(value, &mut with_parameters_declared(target, declared), &mut Scope::new(), classes)
		}
		Node::Key(target, Op::Assign | Op::Define, value) => {
			assignment_checked(node, target, value, declared, lists, classes)
				.or_else(|| check_declared_types(value, declared, lists, classes))
		}
		// `names.add(420)`, `names.insert(420, at: 0)`: the method call appends before mutation lowers it to
		// `names = names + [420]`
		Node::Key(list, Op::Dot, _) if let Some(items) = appended_items(node).or_else(|| inserted_item(node)) => {
			appended_mismatch(node, list, items, declared)
		}
		Node::Key(left, _, right) => check_declared_types(left, declared, lists, classes)
			.or_else(|| check_declared_types(right, declared, lists, classes)),
		_ if crate::tuples::destructuring(node).is_some() => destructured_mismatch(node, declared),
		Node::List(items, bracket, separator) => {
			// `names: list of text = […]` still arrives as the items `names:list`, `of`, `text=[…]`
			if let Some(declaration) = of_type_declaration(items, bracket, separator) {
				return check_declared_types(&declaration, declared, lists, classes);
			}
			let prefixed = items.windows(2).filter_map(|pair| prefixed_declaration(&pair[0], &pair[1]));
			prefixed.chain(items.iter().cloned()).find_map(|item| check_declared_types(&item, declared, lists, classes))
		}
		_ => None,
	}
}

/// `x:int = …` or `x = …` of a declared x: the value must fit the type, which the assignment declares; a list's
/// element type is kept in `lists`, as an undeclared variable's binding
fn assignment_checked(
	node: &Node,
	target: &Node,
	value: &Node,
	declared: &mut HashMap<String, String>,
	lists: &mut Scope,
	classes: &HashSet<String>,
) -> Option<Diagnostic> {
	let declaration = match target.drop_meta() {
		Node::Symbol(name) => declared.get_key_value(name).map(|(name, type_name)| (name.clone(), type_name.clone())),
		declaring => declaring_name_and_type(declaring),
	};
	let Some((name, type_name)) = declaration else {
		if let Node::Symbol(name) = target.drop_meta() {
			let (kind, type_node) = super::super::variables::value_binding(value, lists);
			lists.define(name.clone(), type_node, kind);
		}
		return None;
	};
	if let Some(part) = unsupported_union_part(&type_name, classes) {
		return Some(unknown_union_part(node, &name, &name, &type_name, part));
	}
	declared.insert(name.clone(), type_name.clone());
	let mismatch = assignment_mismatch(node, &name, &type_name, value)
		.or_else(|| evident_items_mismatch(node, &name, &type_name, value, lists));
	if mismatch.is_none() && list_element_type(&type_name).is_some() {
		lists.define(name, Some(Box::new(Node::Symbol(type_name))), Kind::List);
	}
	mismatch
}

/// The items appended to a declared list must fit its element type
fn appended_mismatch(
	node: &Node,
	list: &Node,
	items: &[Node],
	declared: &HashMap<String, String>,
) -> Option<Diagnostic> {
	let Node::Symbol(name) = list.drop_meta() else {
		return None;
	};
	let type_name = declared.get(name)?;
	let element = list_element_type(type_name)?;
	let appended = Node::List(items.to_vec(), Bracket::Square, Separator::Space);
	list_items_mismatch(node, name, type_name, element, &appended)
}

/// The declared types inside a function `(f n:int m)`: its annotated parameters are declared, a plain one hides an outer
/// declaration of its name
pub(super) fn with_parameters_declared(head: &Node, declared: &HashMap<String, String>) -> HashMap<String, String> {
	let mut inner = declared.clone();
	let Node::List(items, _, _) = head.drop_meta() else {
		return inner;
	};
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
pub(super) fn destructured_mismatch(node: &Node, declared: &HashMap<String, String>) -> Option<Diagnostic> {
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
	let Some(expected) = builtin_type_kind(type_name) else {
		// symbol, key, error, time …: a builtin type held as a Node admits a number only among the kinds it holds
		let refused_number = matches!(actual, Kind::Int | Kind::Float)
			&& crate::type_tests::runtime_kind_mask(type_name).is_some_and(|mask| mask & 1 << actual as i64 == 0);
		return !refused_number;
	};
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

/// A list annotation as one type name, so that its name is the list type rather than its first word: `[text]`
/// (named `text` as a list node) and the spaced `list of text` (named `list`)
pub(crate) fn named_list_annotation(annotation: &Node) -> Node {
	match (annotation.drop_meta(), declared_element_type(annotation)) {
		(Node::List(_, Bracket::Square, _), Some(element)) => Node::Symbol(format!("[{element}]")),
		(Node::List(..), Some(element)) => Node::Symbol(format!("{LIST_OF_PREFIX}{element}")),
		_ => annotation.clone(),
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
/// Is the type `word` (an alias too: `double`, `exact`) the type `within` or one it covers (BUILTIN_TYPES)
pub(super) fn is_type_within(word: &str, within: &str) -> bool {
	crate::type_tests::type_matches(crate::type_tests::canonical_spec_word(word), within)
}

pub(crate) fn elements_fit(declared_element: &str, list_type: &str) -> bool {
	let Some(element) = list_type.strip_prefix(LIST_OF_PREFIX) else {
		return true;
	};
	// `list of int or text` (`[1] + ["a"]`): every part must fit
	if let Some(parts) = union_parts(element) {
		return parts.iter().all(|part| elements_fit(declared_element, &format!("{LIST_OF_PREFIX}{part}")));
	}
	// a number int does not cover (rational, real, float …) may be a fraction an int list loses (`rational` names the
	// exact Int representation)
	let fraction = (is_type_within(element, NUMBER_WORD) && !is_type_within(element, INT_WORD)).then_some(Kind::Float);
	match fraction.or_else(|| builtin_type_kind(element)) {
		Some(kind) => admits(declared_element, kind),
		None => true,
	}
}

pub(crate) fn assignment_mismatch(assignment: &Node, name: &str, type_name: &str, value: &Node) -> Option<Diagnostic> {
	// `a = b = 2.5`: an assignment's value is the value it assigns (card chain-unchecked)
	if let Node::Key(_, Op::Assign, assigned) = value.drop_meta() {
		return assignment_mismatch(assignment, name, type_name, assigned);
	}
	// `int? i = "a"`: checked against int, the error names the type as written, "declared int?"
	match (type_name.strip_suffix('?'), value.drop_meta()) {
		(Some(_), Node::Empty) => None,
		(Some(base), _) => base_type_mismatch(assignment, name, base, type_name, value),
		(None, _) => base_type_mismatch(assignment, name, type_name, type_name, value),
	}
}

fn base_type_mismatch(
	assignment: &Node,
	name: &str,
	type_name: &str,
	written: &str,
	value: &Node,
) -> Option<Diagnostic> {
	if value.is_empty_node() && builtin_type_kind(type_name).is_some() {
		let message = format!("type mismatch: {name} is declared {type_name}, cannot assign ø");
		return Some(Diagnostic::at(assignment, message).fix(format!("declare {name}:{type_name}? to allow ø")).offer(
			"allow ø",
			format!("{name}:{type_name}"),
			format!("{name}:{type_name}?"),
		));
	}
	if let Some(element) = list_element_type(type_name) {
		return list_items_mismatch(assignment, name, type_name, element, value);
	}
	let actual = literal_misfit(type_name, value)?;
	let value_text = value.serialize();
	let message = format!(
		"type mismatch: {name} is declared {written}, cannot assign {} {value_text}",
		format!("{actual:?}").to_lowercase()
	);
	let actual = format!("{actual:?}").to_lowercase();
	// a union converts nothing: `int or text(2.5)` is no call, the union grows a part instead
	let widened = match union_parts(type_name) {
		Some(_) => format!("{type_name}{UNION_JOINER}{actual}"),
		None if is_whole_type(type_name) && actual == "float" => NUMBER_WORD.to_string(),
		None => actual.clone(),
	};
	if name == CHECKED_RESULT {
		let truncate = if union_parts(type_name).is_some() {
			String::new()
		} else {
			format!(", or convert with `{value_text} as {type_name}`")
		};
		return Some(
			Diagnostic::at(assignment, message)
				.fix(format!("declare the result {widened} to keep the {actual}{truncate}")),
		);
	}
	if union_parts(type_name).is_some() {
		return Some(Diagnostic::at(assignment, message).fix(format!("declare {name}:{widened}")).offer(
			format!("{name} holds {actual} too"),
			format!("{name}:{type_name}"),
			format!("{name}:{widened}"),
		));
	}
	Some(
		Diagnostic::at(assignment, message)
			.fix(format!("{name}={type_name}({value_text}) or declare {name}:{actual}"))
			.offer(format!("the {type_name} of the value"), &value_text, format!("{type_name}({value_text})"))
			.offer(format!("{name} holds {actual}"), format!("{name}:{type_name}"), format!("{name}:{actual}")),
	)
}

/// `: an int must be a whole number` after the refusal of a fraction for an int, as the run-time check says
/// (list_ops INT_NOT_WHOLE); nothing for any other misfit
pub(crate) fn not_whole_note(type_name: &str, given: Kind) -> &'static str {
	if builtin_type_kind(type_name) == Some(Kind::Int) && given == Kind::Float {
		": an int must be a whole number"
	} else {
		""
	}
}

/// The kind of a literal `value` that does not fit the built-in type `type_name`; None when it fits or is no literal
pub(crate) fn literal_misfit(type_name: &str, value: &Node) -> Option<Kind> {
	if is_bool_type(type_name) {
		return bool_misfit(value);
	}
	let parts = union_parts(type_name).unwrap_or_else(|| vec![type_name]);
	if !parts.iter().all(|part| is_builtin_type(part)) {
		return None;
	}
	// a list literal fits no scalar type: `x: int = [1 2]`
	let list_literal =
		matches!(value.drop_meta(), Node::List(items, Bracket::Square, _) if !items.is_empty()).then_some(Kind::List);
	let actual = list_literal.or_else(|| computed_literal_kind(value))?;
	(!admits(type_name, actual)).then_some(actual)
}

/// A bool is held as an Int, but int ≰ bool: true, false and P199's 1 and 0 (yes and no) fit it, no other int
pub(super) fn bool_misfit(value: &Node) -> Option<Kind> {
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

/// The element type of a list type: `texts`, `list of text` and `[text]` hold `text`
pub(crate) fn list_element_type(type_name: &str) -> Option<&str> {
	plural_element_type(type_name)
		.or_else(|| type_name.strip_prefix(LIST_OF_PREFIX))
		.or_else(|| type_name.strip_prefix('[')?.strip_suffix(']'))
}

/// The items a list value is given literally: all of `[a b]`, the appended ones of `xs + [c]` and of
/// `list_extend(p.xs, [c])` (what `p.xs.add(c)` emits)
pub(crate) fn added_items(value: &Node) -> Option<&[Node]> {
	let appended = match value.drop_meta() {
		Node::Key(_, Op::Add, appended) => appended.as_ref(),
		Node::List(items, Bracket::Round, _) if matches!(items.as_slice(), [call, _, _] if call.drop_meta().name() == crate::wasm_emitter::lists::list_ops::LIST_EXTEND) => {
			&items[2]
		}
		literal => literal,
	};
	match appended.drop_meta() {
		Node::List(items, Bracket::Square, _) => Some(items),
		_ => None,
	}
}

/// `xs = ys`, `xs = xs + ys` of a declared list: the items of a list whose element type is evident (`ys = ["a"]`) must
/// fit, as the run-time check would find (lists `lists` knows the element types of, as written so far)
pub(super) fn evident_items_mismatch(
	assignment: &Node,
	name: &str,
	type_name: &str,
	value: &Node,
	lists: &Scope,
) -> Option<Diagnostic> {
	let element = list_element_type(type_name)?;
	let list_type = list_type_name(value, lists);
	let items = list_type.strip_prefix(LIST_OF_PREFIX).filter(|_| !elements_fit(element, &list_type))?;
	// the value may be a temporary of the element checks (`checked·1`): its items are named, not it
	let message = format!(
		"type mismatch: {name} is declared {type_name}, cannot hold an item that is no {element}: the assigned items are {items}"
	);
	Some(Diagnostic::at(assignment, message).fix(format!("declare {name}:list to hold any items")))
}

/// The first literal item of a list value that does not fit `element`, with its kind: of another kind, or an int
/// outside a fixed width (`xs: [int16] = [70000]`)
pub(crate) fn misfit_item<'a>(element: &str, value: &'a Node) -> Option<(&'a Node, Kind)> {
	added_items(value)?
		.iter()
		.find_map(|item| Some((item, literal_misfit(element, item).or_else(|| out_of_width(element, item))?)))
}

pub(super) fn out_of_width(element: &str, item: &Node) -> Option<Kind> {
	let width = crate::numbers::fixed_width::fixed_width(element)?;
	let Node::Number(Number::Int(value)) = item.drop_meta() else {
		return None;
	};
	(!(width.low..=width.high).contains(&(*value as i128))).then_some(Kind::Int)
}

/// The list variable `names` of an element `names#i`
pub(crate) fn indexed_list(target: &Node) -> Option<&str> {
	let Node::Key(list, Op::Hash, _) = target.drop_meta() else {
		return None;
	};
	list.symbol_name()
}

/// The value of `xs.insert(a, at: i)` (card w0-list-writes: an insert is a write, checked as `xs.add(a)` is)
pub(crate) fn inserted_item(call: &Node) -> Option<&[Node]> {
	let Node::Key(_, Op::Dot, method_call) = call.drop_meta() else {
		return None;
	};
	insert_arguments(method_call).map(|(value, _)| std::slice::from_ref(value))
}

/// The items of an append call `xs.add(a)`, `xs.push(a, b)`
pub(crate) fn appended_items(call: &Node) -> Option<&[Node]> {
	let Node::Key(_, Op::Dot, method_call) = call.drop_meta() else {
		return None;
	};
	let Node::List(items, _, _) = method_call.drop_meta() else {
		return None;
	};
	let (method, arguments) = items.split_first()?;
	let is_append = matches!(method.drop_meta(), Node::Symbol(word) if appends(word, arguments.len()));
	is_append.then_some(arguments)
}

/// `names: texts = [420]`, `names.add(420)`: a literal item the declared element type does not take
pub(super) fn list_items_mismatch(
	assignment: &Node,
	name: &str,
	type_name: &str,
	element: &str,
	value: &Node,
) -> Option<Diagnostic> {
	let (item, actual) = misfit_item(element, value)?;
	let actual = format!("{actual:?}").to_lowercase();
	let message = format!("type mismatch: {name} is declared {type_name}, cannot hold {actual} {}", item.serialize());
	Some(Diagnostic::at(assignment, message).fix(format!("declare {name}:list to hold any items")))
}

/// Kind of a built-in type name (type_tests BUILTIN_TYPES, a fixed width an Int); an exact number (`exact`, `real`) is
/// an Int that may hold a ratio (wasm_emitter/exact.rs)
pub fn builtin_type_kind(name: &str) -> Option<Kind> {
	let name = name.to_lowercase();
	crate::type_tests::held_kind(&name).or_else(|| crate::numbers::fixed_width::fixed_width(&name).map(|_| Kind::Int))
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
				&& !parts.iter().any(|part| builtin_type_kind(part).is_some() && admits(part, Kind::Float))
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
