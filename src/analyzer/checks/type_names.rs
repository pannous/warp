//! Type names: what `type(x)` reports for lists and numbers (`list of int`, `ints`), and the type word all elements fit
use super::*;
use crate::words::{FLOAT_WORD, LIST_WORD, MAP_WORD, NUMBER_WORD, REAL_WORD};

/// A plural type word denotes a list of that type: `ints`, `numbers` → `int`, `number`
pub fn plural_element_type(word: &str) -> Option<&str> {
	let singular = crate::extensions::strings::english_singular(word)?;
	// a type word's plural adds to it (`ints`, `matches`), so the singular is the start of the word
	let singular = word.get(..singular.len()).filter(|start| *start == singular)?;
	type_word_kind(singular).map(|_| singular)
}

/// Code, a quoted program part: `as_code(data e)` (`f.body`, `parse("1+1")`) or a variable bound to one
pub fn is_code(node: &Node, scope: &Scope) -> bool {
	match node.drop_meta() {
		Node::List(items, _, _) => match items.as_slice() {
			[head, _] => head.drop_meta().name() == crate::words::AS_CODE,
			[only] => is_code(only, scope),
			_ => false,
		},
		Node::Symbol(name) => scope
			.lookup(name)
			.and_then(|local| local.type_node.as_deref())
			.is_some_and(|type_node| type_node.drop_meta().name() == crate::words::CODE_WORD),
		_ => false,
	}
}

/// The map type of a variable held as a Node: `m: map = {}`, or `m = {}` given entries (variables.rs type_map_by_first_entry)
pub fn held_map_type(subject: &Node, scope: &Scope) -> Option<String> {
	let local = scope.binding(subject.drop_meta().symbol_name()?)?;
	let type_name = local.type_node.as_ref()?.name();
	(local.kind == Kind::Empty && type_name.starts_with(MAP_WORD)).then_some(type_name)
}

/// The type name `type(x)` reports for a list: a list whose elements are held as Nodes is a plain `list`
pub fn shown_list_type_name(list: &Node, scope: &Scope) -> String {
	let type_name = list_type_name(list, scope);
	if type_name == NODE_LIST_TYPE {
		LIST_WORD.to_string()
	} else {
		type_name
	}
}

/// The type name of a list: `list of int` when all items share a kind (or the variable is declared `ints`), else `list`
pub fn list_type_name(list: &Node, scope: &Scope) -> String {
	match list.ungrouped() {
		Node::Symbol(name) => variable_list_type(name, scope),
		// a block `(out = ø; for …; out)` is worth its last value
		Node::List(items, Bracket::Round | Bracket::Curly, Separator::Semicolon | Separator::Newline)
			if !items.is_empty() =>
		{
			list_type_name(&items[items.len() - 1], scope)
		}
		// `zero_fill(count, zero)`: a list of the zero's type
		Node::List(items, _, _)
			if let [call, _, zero] = items.as_slice()
				&& call.is_symbol(ZERO_FILL_CALL) =>
		{
			format!("{LIST_OF_PREFIX}{}", element_type_word(zero, scope))
		}
		Node::List(items, Bracket::Round, _)
			if let [call, map] = items.as_slice()
				&& let Some(word) = map_word(call) =>
		{
			map_part_type(word, map, scope)
		}
		// `sort(xs)`, `reverse(xs)`: the same elements
		Node::List(items, Bracket::Round, Separator::None)
			if let [order, listed] = items.as_slice()
				&& order.symbol_name().is_some_and(|word| ORDER_WORDS.contains(&word)) =>
		{
			list_type_name(listed, scope)
		}
		Node::List(items, Bracket::Round, Separator::None)
			if let Some(word) = items.first().and_then(Node::symbol_name) =>
		{
			// `split(t, sep)`: texts; `chars(t)`: characters (chars-codepoints)
			if let Some((_, item_type)) = ITEM_TYPED_LIST_WORDS.iter().find(|(list_word, _)| *list_word == word) {
				format!("{LIST_OF_PREFIX}{item_type}")
			} else {
				// the result of a call `f(x)` (no list literal of `f` and `x`): its elements are held as Nodes, of any type
				NODE_LIST_TYPE.to_string()
			}
		}
		// a `{key:value …}` map, `map of int` when all its values are ints
		Node::List(entries, Bracket::Curly, _)
			if !entries.is_empty()
				&& entries.iter().all(|entry| matches!(entry.drop_meta(), Node::Key(_, Op::Colon, _))) =>
		{
			map_type_name(entries, scope)
		}
		// `xs + ys` of two lists: items of their common type, else of the union of two builtin types (held as Nodes)
		Node::Key(left, Op::Add, right) if [left, right].iter().all(|list| infer_type(list, scope) == Kind::List) => {
			joined_list_type(left, right, scope)
		}
		// a value of a map: `graph["A"]` of a `map of list of int` is a `list of int`; an element of a `list of list of int`
		// a `list of int`; an element of a `list of list` (`[("a", 2)]`, tuples of mixed items) holds its items as Nodes
		Node::Key(map, Op::Hash, _) => element_list_type(map, scope),
		Node::List(items, _, separator) => literal_list_type(items, separator, scope),
		_ => LIST_WORD.to_string(),
	}
}

/// A variable's list type: as declared, or a list whose elements are only known at runtime
fn variable_list_type(name: &str, scope: &Scope) -> String {
	let type_name = scope.binding(name).and_then(|local| local.type_node.as_ref()).map(|type_node| type_node.name());
	match type_name {
		Some(word) => match list_element_type(&word) {
			Some(element) => format!("{LIST_OF_PREFIX}{element}"),
			// `xs: list`, a parameter of no declared element type, as an untyped one: each caller may pass other elements
			None if word == LIST_WORD && scope.binding(name).is_some_and(|local| local.is_param) => {
				NODE_LIST_TYPE.to_string()
			}
			None if word.starts_with(LIST_WORD) || word.starts_with(MAP_WORD) => word,
			None => LIST_WORD.to_string(),
		},
		// `r = f()`: the list a function returns, its elements unknown until runtime
		None if scope.binding(name).is_none() && scope.function_kind(name) == Some(Kind::List) => {
			NODE_LIST_TYPE.to_string()
		}
		// a list parameter of no declared element type: each caller may pass other elements
		None if scope.binding(name).is_some_and(|local| local.is_param) => NODE_LIST_TYPE.to_string(),
		None => LIST_WORD.to_string(),
	}
}

/// `keys(m)`, `entries(m)`, `values(m)`: texts, the map's own type, or a list of its value type
fn map_part_type(word: &str, map: &Node, scope: &Scope) -> String {
	use crate::library_words::MAP_ENTRIES;
	use crate::words::MAP_KEYS;
	match word {
		MAP_KEYS => format!("{LIST_OF_PREFIX}text"), // the keys of a map are read as texts
		MAP_ENTRIES => list_type_name(map, scope),
		// the values of a map of known values; of anything else (a pair `a: {b: 8}`) Nodes of any type
		_ => match list_type_name(map, scope) {
			map if map.starts_with(MAP_WORD) => map.replacen(MAP_TYPE_PREFIX, LIST_OF_PREFIX, 1),
			_ => NODE_LIST_TYPE.to_string(),
		},
	}
}

fn map_type_name(entries: &[Node], scope: &Scope) -> String {
	let values: Vec<&Node> = entries
		.iter()
		.filter_map(|entry| match entry.drop_meta() {
			Node::Key(_, _, value) => Some(value.as_ref()),
			_ => None,
		})
		.collect();
	// an empty `[]` fits any type of list: `{A:[["B", 4]], B:[]}` is a `map of list of list`
	let is_empty_list = |value: &&Node| {
		value.is_empty_node() || matches!(value.drop_meta(), Node::List(items, Bracket::Square, _) if items.is_empty())
	};
	let typed: Vec<&Node> = values.iter().copied().filter(|value| !is_empty_list(value)).collect();
	let words: Vec<String> =
		if typed.is_empty() { &values } else { &typed }.iter().map(|value| nested_type_word(value, scope)).collect();
	let is_list_map = typed.len() < values.len() && words.iter().all(|word| word.starts_with(LIST_WORD));
	match common_type_word(&words) {
		Some(word) if is_list_map || typed.len() == values.len() => format!("{MAP_TYPE_PREFIX}{word}"),
		_ => MAP_WORD.to_string(),
	}
}

fn joined_list_type(left: &Node, right: &Node, scope: &Scope) -> String {
	let elements = [left, right].map(|list| list_type_name(list, scope));
	let words: Option<Vec<String>> =
		elements.iter().map(|list| list.strip_prefix(LIST_OF_PREFIX).map(str::to_string)).collect();
	let Some(words) = words else {
		return LIST_WORD.to_string();
	};
	match common_type_word(&words) {
		Some(word) => format!("{LIST_OF_PREFIX}{word}"),
		None if words.iter().all(|word| is_builtin_or_union(word)) => {
			format!("{LIST_OF_PREFIX}{}", words.join(UNION_JOINER))
		}
		None => NODE_LIST_TYPE.to_string(),
	}
}

/// A list value of a map or list: see list_type_name
fn element_list_type(map: &Node, scope: &Scope) -> String {
	let outer = list_type_name(map, scope);
	match outer.strip_prefix(MAP_TYPE_PREFIX).or_else(|| outer.strip_prefix(LIST_OF_PREFIX)) {
		Some(LIST_WORD) if outer.starts_with(LIST_OF_PREFIX) => NODE_LIST_TYPE.to_string(),
		Some(value_type) if value_type.starts_with(LIST_WORD) => value_type.to_string(),
		_ => LIST_WORD.to_string(),
	}
}

/// A list of items of a common type, else of Nodes; a block of no common type is a plain list
fn literal_list_type(items: &[Node], separator: &Separator, scope: &Scope) -> String {
	let words: Vec<String> = items.iter().map(|item| nested_type_word(item, scope)).collect();
	let is_block = matches!(separator, Separator::Semicolon | Separator::Newline);
	match common_type_word(&words) {
		Some(word) => format!("{LIST_OF_PREFIX}{word}"),
		// `[1 "a"]`: elements of different kinds, each held as its Node
		None if !words.is_empty() && !is_block => NODE_LIST_TYPE.to_string(),
		None => LIST_WORD.to_string(),
	}
}

/// The type word of an element: an item that is a list keeps its own element type, `[(0, 1), (1, 2)]` is a
/// `list of list of int`
pub(super) fn nested_type_word(item: &Node, scope: &Scope) -> String {
	match infer_type(item, scope) {
		Kind::List => list_type_name(item, scope),
		_ => element_type_word(item, scope),
	}
}

/// Library words whose list result has the elements of their list argument
pub(in crate::analyzer) const ORDER_WORDS: [&str; 2] = ["sort", "reverse"];
/// Library words whose result is a list, with the type of its items
pub(super) const ITEM_TYPED_LIST_WORDS: [(&str, &str); 2] = [("split", "text"), ("chars", "codepoint")];
pub(in crate::analyzer) const MAP_TYPE_PREFIX: &str = "map of ";
/// A list whose elements are known only at runtime (the result of a call): each element is held as a Node
pub(in crate::analyzer) const NODE_LIST_TYPE: &str = "list of node";
/// Between a variable and the name of a temporary made for it (`xs·range`, `xs·item`, `m·removed`)
pub const TEMPORARY_SEPARATOR: &str = "·";

/// A name only a lowering pass makes (`word·sum·3`, `x·iterator`): the program cannot write `·` in a name, where it
/// reads as a product
pub fn is_compiler_temporary(name: &str) -> bool {
	name.contains(TEMPORARY_SEPARATOR)
}
pub(crate) const LIST_OF_PREFIX: &str = "list of ";

/// The map word a call names: `map_keys`, `map_values` or `map_entries`
pub(in crate::analyzer) fn map_word(call: &Node) -> Option<&'static str> {
	use crate::library_words::{MAP_ENTRIES, MAP_VALUES};
	use crate::words::MAP_KEYS;
	[MAP_KEYS, MAP_VALUES, MAP_ENTRIES].into_iter().find(|word| call.is_symbol(word))
}

pub(in crate::analyzer) const INT_WORD: &str = "int";
pub(crate) const RATIONAL_WORD: &str = "rational";
pub(in crate::analyzer) const REAL_CONSTANTS: [&str; 2] = ["π", "pi"];

/// The type word of a number literal: whole numbers are `int` (also `2.0`), exact fractions and decimals `rational`,
/// approximations (`1.5f`, √2, complex) `float` (decision exact-decimals)
pub fn number_type_word(number: &Number) -> &'static str {
	match number {
		Number::Int(_) | Number::BigInt(_) => INT_WORD,
		Number::Quotient(..) | Number::BigQuotient(_) => RATIONAL_WORD,
		Number::Real(_) => REAL_WORD,
		Number::Float(value) if Number::is_exact_decimal(*value) => {
			if value.fract() == 0.0 {
				INT_WORD
			} else {
				RATIONAL_WORD
			}
		}
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

pub(in crate::analyzer) fn element_type_word(item: &Node, scope: &Scope) -> String {
	if item.is_empty_node() {
		return crate::type_tests::EMPTY_TYPE.to_string();
	}
	literal_number_type_word(item).map(str::to_string).unwrap_or_else(|| infer_type(item, scope).to_string())
}

/// The one type word all element words fit: the same word, `rational` for a mix of `int` and `rational` (int is a special
/// case of rational), `number` for any other mix of numbers, optional (`int?`) when some are ø or optional (P179);
/// `None` when the elements are not all numbers or all alike
pub(in crate::analyzer) fn common_type_word(words: &[String]) -> Option<String> {
	let is_optional = |word: &String| word == crate::type_tests::EMPTY_TYPE || word.ends_with('?');
	if words.iter().any(is_optional) {
		let values: Vec<String> = words
			.iter()
			.filter(|word| *word != crate::type_tests::EMPTY_TYPE)
			.map(|word| word.trim_end_matches('?').to_string())
			.collect();
		return match values.is_empty() {
			true => Some(crate::type_tests::EMPTY_TYPE.to_string()),
			false => common_type_word(&values).map(|word| format!("{word}?")),
		};
	}
	let first = words.first()?;
	if words.iter().all(|word| word == first) {
		return Some(first.clone());
	}
	let is_number = |word: &String| is_type_within(word, NUMBER_WORD);
	if !words.iter().all(is_number) {
		return None;
	}
	Some(
		if words.iter().all(|word| is_type_within(word, RATIONAL_WORD)) { RATIONAL_WORD } else { NUMBER_WORD }
			.to_string(),
	)
}
