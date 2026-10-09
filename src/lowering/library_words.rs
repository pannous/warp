//! The basic text and list words (`upper`, `first`, `sum`, `split` …) in their three spellings: `x.word`, `x.word(args)`,
//! `word(x, args)` and `word x args` all become the call `word(x, args)`.
//! Words that need no runtime function are expanded to source here (`first`, `last`, `sum`); the others are calls that the
//! emitter resolves (`RUNTIME_WORDS`). A user function or variable of the same name wins.
//!
//! An unknown `.word` after a name, a text or a list is a loud error (`undefined function: word`), never silent data.

use crate::analyzer::{call_name, counting_method, extract_user_functions, is_list_mutating_method};
use crate::context::Context;
use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::{parse, ASSERT_MARKER, TRY_MARKER};
use crate::wasm_emitter::{CAUGHT_ERROR, RAN_WITHOUT_ERROR};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

/// The map words, named after their runtime functions: `m.keys()`, `m.values()`, `x in xs` (`xs.has(x)`, Python's and
/// JavaScript's spellings too) and `m.get(k)`/`m.get(k, default)`. A key is found by its letters, quoted or not.
pub const MAP_KEYS: &str = "map_keys";
pub const MAP_VALUES: &str = "map_values";
/// map_entries(xs): the `key:value` entries of a map as a list (a one-entry map `{a:1}` is the entry itself), else xs
pub const MAP_ENTRIES: &str = "map_entries";
pub const COLLECTION_CONTAINS: &str = "collection_contains";
/// collection_position(xs, x): `x in xs`, the 1-based position of x in a list (0 when absent), 1/0 for a map key
pub const COLLECTION_POSITION: &str = "collection_position";
pub const MAP_GET_OR: &str = "map_get_or";
/// map_without(map, key): the map without the key's entry, what `m.remove(k)` stores in m (analyzer removed_key)
pub const MAP_WITHOUT: &str = "map_without";
pub const MAP_WORD_FUNCTIONS: [&str; 7] = [MAP_KEYS, MAP_VALUES, MAP_ENTRIES, COLLECTION_CONTAINS, COLLECTION_POSITION, MAP_GET_OR, MAP_WITHOUT];
const IN_WORD: &str = "in";
/// `count x in y` (count_in): the occurrences of an item or a character, or of a substring in a text
const COUNT_WORD: &str = "count";
const FLOAT_WORD: &str = "float";
/// The name `catch e` binds, in lower_try_binding's template
const CAUGHT_BINDING_PLACEHOLDER: &str = "caught_binding";
/// The got-it topic of `square 3 + square(4)` (ask_braceless_argument)
const BRACELESS_ARGUMENT_TOPIC: &str = "braceless-call-operator";
const COUNT_HAYSTACK: &str = "counted_haystack";
const COUNT_NEEDLE: &str = "counted_needle";
const COUNT_ITEM_TEMPLATE: &str = "count(filter(counted_haystack, counted_item => counted_item == counted_needle))";
const COUNT_SUBSTRING_TEMPLATE: &str = "count(split(counted_haystack, counted_needle)) - 1";
const FOR_WORD: &str = "for";
/// `log(x)` is libm's natural logarithm; `log(x, base)` divides by the base's
const LOG_WORD: &str = "log";
const OF_WORD: &str = "of";

/// Canonical word and the spellings that mean it
const SYNONYMS: [(&str, &[&str]); 26] = [
	(LIST_WORD, &[]),
	(MAP_KEYS, &["keys"]),
	(MAP_VALUES, &["values"]),
	(MAP_ENTRIES, &[]),
	(COLLECTION_CONTAINS, &["contains", "has", "includes"]),
	(MAP_GET_OR, &["get"]),
	// P35 default: `xs.index_of(x)` is the 1-based position, as `x in xs` (0 when absent)
	(COLLECTION_POSITION, &["index_of"]),
	("chars", &[]),
	("upper", &["uppercase"]),
	("lower", &["lowercase"]),
	// the text builtin trim (text_builtins.rs), Python's strip
	("trim", &["strip"]),
	("reverse", &[]),
	("sort", &["sorted"]),
	("split", &[]),
	("join", &[]),
	("first", &[]),
	("last", &[]),
	(SLICE, &[]),
	(COPY, &["clone"]),
	("replace", &[]),
	(ORD, &["ordinal", CODEPOINT]),
	(IS_DIGIT, &["isdigit"]),
	("mean", &["average"]),
	(IS_ALPHA, &["is_letter", "isalpha"]),
	("is_alphanumeric", &["is_alnum", "isalnum"]),
	(crate::mutation::UNWRAP, &[]),
];
const SUM: &str = "sum";
/// `list_sum(list, loop)`: the sum of a list variable as one operation the emitter dispatches (wasm_emitter/list_dispatch.rs):
/// a typed list sums its array, any other list runs the loop `sum` always lowered to
pub const LIST_SUM: &str = "list_sum";

/// Words the emitter implements as runtime functions, with the number of arguments including the receiver
pub const RUNTIME_WORDS: [(&str, usize); 19] = [
	(crate::wasm_emitter::wasi_emitter::WASI_ENVIRONMENT, 0),
	(ORD, 1), ("upper", 1), ("lower", 1), ("reverse", 1), ("sort", 1), ("split", 2), ("join", 2), ("chars", 1), (FIELD_WITH, 3),
	(MAP_KEYS, 1), (MAP_VALUES, 1), (MAP_ENTRIES, 1), (COLLECTION_CONTAINS, 2), (COLLECTION_POSITION, 2), (MAP_GET_OR, 3), (SLICE, 3),
	(MAP_WITHOUT, 2), (INSTANCE_COPY, 2),
];
/// `codepoint(c)`, `ord(c)`, `ordinal(c)`: the code point of a character (`c as int` is only its digit). Inside the
/// compiler it is `ord`, since `codepoint` is also a type word
pub const ORD: &str = "ord";
/// P74 (user, 2026-10-05): "make codepoint the default name": the spelling hints, fixes and docs use
pub const CODEPOINT: &str = "codepoint";
/// `slice(x, start, end)`: the items or characters start…end-1, 0-based (`a[1:3]`, `s.slice(1)`)
pub const SLICE: &str = "slice";
/// Trailing arguments a word may leave out, passed as ø: `m.get(k)` is ø for a missing key, `s.slice(2)` slices to the end;
/// `s.split` splits at spaces (left_out_argument)
const OPTIONAL_ARGUMENTS: [(&str, usize); 3] = [(MAP_GET_OR, 1), (SLICE, 1), (SPLIT, 1)];
const SPLIT: &str = "split";
const SPLIT_SEPARATOR: &str = " ";
/// What an optional argument left out is: the space `s.split` splits at, else ø
fn left_out_argument(word: &str) -> Node {
	match word {
		SPLIT => Node::Text(SPLIT_SEPARATOR.to_string()),
		_ => Node::Empty,
	}
}

/// `b = a.copy()`: values are never shared, so the copy is the value itself
const COPY: &str = "copy";
/// `field_with(object, "name", value)`: a copy of the object with the field set; what `object.name = value` lowers to
pub const FIELD_WITH: &str = "field_with";
/// instance_copy(x): a new instance with x's field values (`p.copy()`, Kotlin's `p.copy(y = 5)`); any other value is x
pub const INSTANCE_COPY: &str = "instance_copy";
/// The named argument of `copy` asking for a shallow copy (P205)
pub const SHALLOW: &str = "shallow";

/// `instance_copy(object, shallow)`
pub fn copy_call(object: Node, shallow: Node) -> Node {
	Node::List(vec![Node::Symbol(INSTANCE_COPY.to_string()), object, shallow], Bracket::Round, Separator::None)
}

/// The value of `shallow: v` / `shallow = v`, or a positional `v`
pub fn shallow_flag(argument: &Node) -> Node {
	match argument.drop_meta() {
		Node::Key(name, Op::Colon | Op::Assign, value) if name.drop_meta().name() == SHALLOW => value.as_ref().clone(),
		other => other.clone(),
	}
}

/// Source of the words expanded here, with their number of arguments; `word_argument` is the receiver, `word_tmp` a
/// temporary that holds it once, `word_argument_2` … the arguments after the receiver. The words written in warp
/// (first, last, round_to, replace, is_digit …) are lib/prelude.warp's; these three the emitter dispatches on
const EXPANDED_WORDS: [(&str, usize, &str); 3] = [
	// Python's `list(x)`: the list itself, a text's characters
	(LIST_WORD, 1, "word_tmp as list"),
	(SUM, 1, "(word_sum=0; for word_item in word_tmp {word_sum = word_sum + word_item}; word_sum)"),
	// `x!` (mutation.rs): the value, a loud error when it is ø; an Error value stays that Error
	(crate::mutation::UNWRAP, 1, "if word_tmp == ø then error(\"unwrapped ø\") else word_tmp"),
];
const IS_DIGIT: &str = "is_digit";
const LIST_WORD: &str = "list";
/// Conversions a method may name, `x.string` being `string(x)`
const CONVERSION_METHODS: [&str; 4] = ["string", "str", "int", "float"];
const LIST_CONSTRUCTORS: [&str; 4] = ["listOf", "mutableListOf", "arrayOf", "arrayListOf"];
/// Words that mean another word with that many arguments (receiver included): `round(x, 3)`, `x.round(3)` is x rounded
/// to 3 digits after the point (`round(x)` stays the builtin); `xs.first(3)`, `first 3 of xs` the first 3 items,
/// `last(xs, 2)` the last 2 (lib/prelude.warp)
const ARITY_VARIANTS: [(&str, usize, &str); 3] = [("round", 2, "round_to"), ("first", 2, "first_items"), ("last", 2, "last_items")];
const IS_ALPHA: &str = "is_alpha";
/// Hidden variables and placeholders of the `try`/`assert` templates
const TRY_TEMPORARY: &str = "try_tmp";
const TRY_VALUE_PLACEHOLDER: &str = "try_placeholder_value";
const TRY_FALLBACK_PLACEHOLDER: &str = "try_placeholder_fallback";
const LIST_PLACEHOLDER: &str = "try_placeholder_list";
const INDEX_PLACEHOLDER: &str = "try_placeholder_index";
const DIVIDEND_PLACEHOLDER: &str = "try_placeholder_dividend";
const DIVISOR_PLACEHOLDER: &str = "try_placeholder_divisor";
const ASSERT_CONDITION_PLACEHOLDER: &str = "assert_placeholder_condition";
const LEFT_OPERAND_PLACEHOLDER: &str = "operand_placeholder_left";
const RIGHT_OPERAND_PLACEHOLDER: &str = "operand_placeholder_right";
const OPERAND_TEMPORARY: &str = "operand_tmp";
/// The variable holding a list while one of its elements changes: `place·0`
const PLACE_TEMPORARY: &str = "place·";
/// values_similar(a, b, tolerance): `a ≈ b` (wasm_emitter/similarity.rs)
pub const VALUES_SIMILAR: &str = "values_similar";
/// values_rough(a, b, tolerance): `a ~ b`, looser (P211)
pub const VALUES_ROUGH: &str = "values_rough";
/// Per level of approximation: its operator, its call, the variable a program sets its relative tolerance with
/// (|a-b| ≤ tolerance·max(|a|, |b|)) and the default tolerance
/// values_similar or values_rough
pub fn is_similarity_call(name: &str) -> bool {
	SIMILARITY_LEVELS.iter().any(|(_, function, ..)| *function == name)
}

pub const SIMILARITY_LEVELS: [(Op, &str, &str, &str); 2] = [(Op::Similar, VALUES_SIMILAR, "tolerance", "1e-9"), (Op::Rough, VALUES_ROUGH, "rough_tolerance", "0.01")];
const RECEIVER_PLACEHOLDER: &str = "word_argument";
const LOOKUP_PLACEHOLDER: &str = "word_lookup";
const TEMPORARY: &str = "word_tmp";

/// Library words whose result is always a text, and those whose result is always a list
const TEXT_RESULT_WORDS: [&str; 5] = ["upper", "lower", "trim", "join", crate::wasm_emitter::wasi_emitter::WASI_ENVIRONMENT];
const LIST_RESULT_WORDS: [&str; 8] = ["chars", "sort", "split", MAP_KEYS, MAP_VALUES, MAP_ENTRIES, MAP_WITHOUT, crate::wasm_emitter::list_ops::LIST_EXTEND];

pub fn result_kind(word: &str) -> Option<crate::type_kinds::Kind> {
	use crate::type_kinds::Kind;
	if TEXT_RESULT_WORDS.contains(&word) {
		Some(Kind::Text)
	} else {
		LIST_RESULT_WORDS.contains(&word).then_some(Kind::List)
	}
}

/// `field_with(m, key, value)` updating the variable m itself: what `m.key = value` lowers to (field_assignment)
pub fn is_field_update_of(name: &str, value: &Node) -> bool {
	let Node::List(items, _, _) = value.drop_meta() else { return false };
	let [word, map, _, _] = items.as_slice() else { return false };
	word.name() == FIELD_WITH && matches!(map.drop_meta(), Node::Symbol(map) if map == name)
}

pub fn is_runtime_word(name: &str) -> bool {
	RUNTIME_WORDS.iter().any(|(word, _)| *word == name)
}

/// A word of the library, under any of its spellings: `codepoint(c)` calls it, it is no type constructor
pub fn is_library_word(name: &str) -> bool {
	is_runtime_word(name) || canonical_word(name).is_some()
}

/// Words that take the name after them as their argument inside an operand, as a defined function does:
/// `word == reverse word` compares with `reverse(word)`
const OPERAND_WORDS: [&str; 7] = ["reverse", "sort", "upper", "lower", "trim", "chars", SUM];

pub fn is_operand_word(name: &str) -> bool {
	canonical_word(name).is_some_and(|word| OPERAND_WORDS.contains(&word))
}

fn canonical_word(name: &str) -> Option<&'static str> {
	SYNONYMS
		.iter()
		.find(|(word, synonyms)| *word == name || synonyms.contains(&name))
		.map(|(word, _)| *word)
		.or((name == SUM).then_some(SUM))
}

fn arity(word: &str) -> usize {
	let expanded = EXPANDED_WORDS.iter().map(|(name, arity, _)| (*name, *arity));
	let built_in = RUNTIME_WORDS.into_iter().chain(expanded).find(|(name, _)| *name == word).map(|(_, arity)| arity);
	built_in.or_else(|| crate::modules::prelude_word_arity(word)).unwrap_or(1)
}

/// The library words a name may stand for once this pass has run: its canonical spelling (`isdigit` → is_digit) and,
/// its ARITY_VARIANTS (`round(x, 2)` round_to). modules::resolve, which runs before, loads the prelude words by them
pub fn words_spelled_by(name: &str) -> Vec<&'static str> {
	let variants = ARITY_VARIANTS.iter().filter(|(word, _, _)| *word == name).map(|(_, _, variant)| *variant);
	canonical_word(name).into_iter().chain(variants).collect()
}

pub fn lower(node: Node) -> Node {
	let node = bind_read_data_objects(node);
	let mut context = Context::new();
	extract_user_functions(&mut context, &node);
	let mut shadowed: HashSet<String> = context.user_functions.keys().cloned().collect();
	// `sum·point`, a class's own `sum` (traits.rs witness, an overload): the program defines `sum`, no library word
	let variants: Vec<String> = shadowed.iter().filter_map(|name| name.split_once(crate::traits::WITNESS_SEPARATOR)).map(|(operation, _)| operation.to_string()).collect();
	shadowed.extend(variants);
	collect_assigned_names(&node, &mut shadowed);
	let mut assigned = AssignedObjects::default();
	collect_assigned_objects(&node, &mut assigned);

	let plain = assigned.plain.difference(&assigned.rebound).cloned().collect();
	let objects = assigned.objects.into_iter().filter_map(|(name, literal)| Some((name, literal?))).collect();
	let instances = crate::traits::InstanceTypes::of(&node);
	let call_results = call_results(&node, &context);
	Lowering { context, shadowed, objects, plain, instances, parameters: RefCell::new(vec![]), defining: RefCell::new(vec![]), call_results, temporaries: Cell::new(0) }.expand(node)
}

/// The variables assigned what a user function returns (`result = parse_json(text)`): any may hold an object
fn call_results(node: &Node, context: &Context) -> HashSet<String> {
	let mut names = HashSet::new();
	node.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = part {
			let calls_user_function = matches!(value.drop_meta(), Node::List(items, Bracket::Round, _)
				if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(function)) if context.user_functions.contains_key(function)));
			if let (Node::Symbol(name), true) = (target.drop_meta(), calls_user_function) {
				names.insert(name.clone());
			}
		}
	});
	names
}

/// `raise X`, `throw X`, `raise(X)`: the call `raise(X)`; `raise error("m")` raises the message m
fn raise_call(items: &[Node]) -> Option<Node> {
	let [word, raised] = items else { return None };
	if !crate::pipeline::RAISE_WORDS.iter().any(|raise| is_marker(word, raise)) {
		return None;
	}
	let message = crate::pipeline::returned_error_message(raised).unwrap_or(raised).clone();
	// the message is a text, so `catch e` reads it whatever was raised (`raise 42`: "42"); a word (`raise oops`) is its name
	let message = match message.drop_meta() {
		Node::Text(_) | Node::Symbol(_) => message,
		_ => Node::List(vec![Node::Symbol(crate::wasm_emitter::text_builtins::TEXT_FORM.to_string()), message], Bracket::Round, Separator::None),
	};
	Some(Node::List(vec![Node::Symbol(crate::wasm_emitter::text_builtins::RAISE.to_string()), message], Bracket::Round, Separator::None))
}

/// The pass of `count x in y` (count_in), before the lambdas its rewrite uses are lowered and before `x in y` is membership;
/// also `y.count(x)` and `count(y, x)` (card banana-count) unless the program defines its own count
pub fn lower_count_in(node: Node) -> Node {
	let mut found = Vec::new();
	crate::function_values::definitions(&node, &mut found);
	let library_count = !found.iter().any(|definition| definition.name == COUNT_WORD);
	lower_counts(node, library_count)
}

fn lower_counts(node: Node, library_count: bool) -> Node {
	let counted = |node: &Node| match node {
		Node::List(items, bracket, _) => count_in(items).or_else(|| library_count.then(|| count_call(items, bracket)).flatten()),
		Node::Key(receiver, Op::Dot, call) if library_count => count_method(receiver, call),
		_ => None,
	};
	if let Some(occurrences) = counted(&node) {
		return lower_counts(occurrences, library_count);
	}
	match node {
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| lower_counts(item, library_count)).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(lower_counts(*left, library_count)), op, Box::new(lower_counts(*right, library_count))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower_counts(*node, library_count)), data },
		other => other,
	}
}

/// `count(y, x)` is `count x in y`
fn count_call(items: &[Node], bracket: &Bracket) -> Option<Node> {
	let [count, haystack, needle] = items else { return None };
	(*bracket == Bracket::Round && is_marker(count, COUNT_WORD)).then(|| counted_in(count, needle, haystack)).flatten()
}

/// `y.count(x)` is `count x in y`
fn count_method(receiver: &Node, call: &Node) -> Option<Node> {
	let Node::List(items, _, _) = call.drop_meta() else { return None };
	let [count, needle] = items.as_slice() else { return None };
	is_marker(count, COUNT_WORD).then(|| counted_in(count, needle, receiver)).flatten()
}

fn counted_in(count: &Node, needle: &Node, haystack: &Node) -> Option<Node> {
	count_in(&[count.clone(), needle.clone(), Node::Symbol(IN_WORD.to_string()), haystack.clone()])
}

/// `count x in y`: how often x occurs in y. A text of several characters counts as a substring of a text (`count "an"
/// in "banana"` → 2), anything else as an item of a list or a character of a text; `count bytes in t` is `t.bytes`
fn count_in(items: &[Node]) -> Option<Node> {
	// `count 'a' in s` arrives as the words or as `count ('a' in s)`
	let phrase: Vec<Node> = match items {
		[count, phrase] => match phrase.drop_meta() {
			Node::List(rest, Bracket::None, _) => [vec![count.clone()], rest.clone()].concat(),
			_ => return None,
		},
		_ => items.to_vec(),
	};
	let [count, needle, in_word, haystack @ ..] = phrase.as_slice() else { return None };
	if !is_marker(count, COUNT_WORD) || !is_marker(in_word, IN_WORD) || haystack.is_empty() {
		return None;
	}
	let haystack = match haystack {
		[single] => single.clone(),
		several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
	};
	if let Some(unit) = matches!(needle.drop_meta(), Node::Symbol(_)).then(|| crate::analyzer::text_unit(&needle.name())).flatten() {
		return Some(Node::Key(Box::new(haystack), Op::Dot, Box::new(Node::Symbol(unit.to_string()))));
	}
	let substring = matches!(needle.drop_meta(), Node::Text(text) if text.chars().count() > 1) && !matches!(haystack.drop_meta(), Node::List(_, Bracket::Square, _));
	let template = if substring { COUNT_SUBSTRING_TEMPLATE } else { COUNT_ITEM_TEMPLATE };
	Some(substitute(substitute(parse(template), COUNT_HAYSTACK, &haystack), COUNT_NEEDLE, needle))
}

/// Method syntax for builtin functions: the called method `x.f(args)` is `f(x, args)` when f is a rounding or libm
/// function the program does not define. Runs first, so the passes that know these calls see their plain form; user
/// functions are called so later, in `method_call`
pub fn lower_function_methods(node: Node) -> Node {
	let mut context = Context::new();
	extract_user_functions(&mut context, &node);
	let arities: HashMap<String, usize> = context.user_functions.iter().map(|(name, function)| (name.clone(), function.params.len())).collect();
	let mut defined: HashSet<String> = arities.keys().cloned().collect();
	collect_assigned_names(&node, &mut defined);
	// a libc function is no method: `m.remove(k)` is the map's, not stdio's remove(path)
	let is_builtin = |name: &str| crate::wasm_emitter::ROUNDING_FUNCTIONS.contains(&name) || crate::ffi::get_ffi_signature(name).is_some_and(|signature| signature.library != "c");
	// `xs.map(square)` of a user function map(list, fn): the receiver is its first argument, before function values
	// are specialised (function_values.rs), which would otherwise see map(square)
	let takes_receiver = |name: &str, arguments: usize| arities.get(name) == Some(&(arguments + 1));
	let node = list_constructor_calls(node, &defined);
	function_methods_as_calls(node, &|name, arguments| (is_builtin(name) && !defined.contains(name)) || takes_receiver(name, arguments))
}

/// Python's `list()`: the empty list, as `[]` parses; Kotlin's `listOf(1, 2)`, `arrayOf`, `mutableListOf`: the list
/// `[1, 2]` (`listOf()` is `[]` too); a word the program defines keeps its call
fn list_constructor_calls(node: Node, defined: &HashSet<String>) -> Node {
	let is_constructor = |word: &Node| matches!(word.drop_meta(), Node::Symbol(name) if (name == LIST_WORD || LIST_CONSTRUCTORS.contains(&name.as_str())) && !defined.contains(name));
	match node {
		Node::List(items, Bracket::Round, Separator::None) if items.first().is_some_and(is_constructor) => {
			let elements: Vec<Node> = match &items[1..] {
				[Node::List(arguments, Bracket::Round, _)] => arguments.clone(),
				arguments => arguments.to_vec(),
			};
			let is_list_word = matches!(items[0].drop_meta(), Node::Symbol(name) if name == LIST_WORD);
			match elements.is_empty() {
				true => Node::Empty,
				false if is_list_word => Node::List(items.into_iter().map(|item| list_constructor_calls(item, defined)).collect(), Bracket::Round, Separator::None), // `list(x)` is `x as list`
				false => Node::List(elements.into_iter().map(|element| list_constructor_calls(element, defined)).collect(), Bracket::Square, Separator::Colon),
			}
		}
		other => other.map_children(|child| list_constructor_calls(child, defined)),
	}
}

fn function_methods_as_calls(node: Node, is_function: &dyn Fn(&str, usize) -> bool) -> Node {
	let lowered = |node: Node| function_methods_as_calls(node, is_function);
	match node {
		Node::Key(receiver, Op::Dot, method) => {
			let (receiver, method) = (lowered(*receiver), lowered(*method));
			match method.drop_meta() {
				Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if is_function(name, items.len() - 1)) => {
					Node::List([vec![items[0].clone(), receiver], items[1..].to_vec()].concat(), Bracket::Round, Separator::None)
				}
				_ => Node::Key(Box::new(receiver), Op::Dot, Box::new(method)),
			}
		}
		Node::Key(left, op, right) => Node::Key(Box::new(lowered(*left)), op, Box::new(lowered(*right))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lowered).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lowered(*node)), data },
		other => other,
	}
}

/// The field and step of `x++` / `x--` after a dot
fn field_step(method: &Node) -> Option<(Node, Op)> {
	match method.drop_meta() {
		Node::Key(field, op @ (Op::Inc | Op::Dec), _) if matches!(field.drop_meta(), Node::Symbol(_)) => Some((field.as_ref().clone(), *op)),
		_ => None,
	}
}

/// A place deeper than a variable, its element or its field (`b.n += 1` sets a struct field in place): `p.b.n`,
/// `bags#1.n`, `xs#1#2`
fn is_nested_place(place: &Node) -> bool {
	match place.drop_meta() {
		Node::Key(base, Op::Dot | Op::Hash, _) => !matches!(base.drop_meta(), Node::Symbol(_)),
		_ => false,
	}
}

/// `place += 1` for `place++`, `place -= 1` for `place--`
pub(crate) fn stepped(place: Node, step: Op) -> Node {
	let update = if step == Op::Inc { Op::AddAssign } else { Op::SubAssign };
	Node::Key(Box::new(place), update, Box::new(Node::int(1)))
}

/// `object.name`, `name of object`, `object["name"]`: the entry named `name`, a subscript by a text key
fn field_lookup(object: &Node, name: &str, position: &Node) -> Node {
	let key = match position {
		Node::Meta { data, .. } => Node::Meta { node: Box::new(Node::Text(name.to_string())), data: data.clone() },
		_ => Node::Text(name.to_string()),
	};
	crate::warp_parser::subscript(object.clone(), key)
}

/// A field lookup by a name, as `field_lookup` builds it: its value is an object whenever the field holds one
/// A field read by name (`p.x`, `p["x"]`), an element of one (`company.employees#2`), or either in parentheses: the
/// value may be an object
fn is_field_lookup(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, Op::Hash, index) if crate::warp_parser::subscript_key(index).and_then(field_name).is_some() => true,
		Node::Key(list, Op::Hash, _) => is_field_lookup(list),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => is_field_lookup(&items[0]),
		_ => false,
	}
}

/// The entries of an object literal `{a:1 b:2}` (or `{a:1}`): every item is a `key:value`
pub(crate) fn object_entries(node: &Node) -> Option<Vec<(String, Node)>> {
	let single;
	let items: &[Node] = match node.drop_meta() {
		Node::List(items, Bracket::Curly, _) => items, // `{}` is the empty object, grown by `d["k"] = v`
		// a named tuple `(x: 10, y: 20)`: its entries are fields too
		Node::List(items, Bracket::Round, _) if items.len() > 1 && items.iter().all(|item| matches!(item.drop_meta(), Node::Key(_, Op::Colon, _))) => items,
		Node::Key(_, Op::Colon, _) => {
			single = [node.clone()];
			&single
		}
		_ => return None,
	};
	items
		.iter()
		.map(|item| match item.drop_meta() {
			Node::Key(key, Op::Colon, value) => Some((field_name(key)?, value.as_ref().clone())),
			_ => None,
		})
		.collect()
}

/// The field name of a subscript `p["name"]`, as `p.name` lowers: None for an element `xs#i`
fn subscript_field(index: &Node) -> Option<String> {
	match crate::warp_parser::subscript_key(index)?.drop_meta() {
		Node::Text(name) => Some(name.clone()),
		Node::Char(letter) => Some(letter.to_string()),
		_ => None,
	}
}

fn field_name(key: &Node) -> Option<String> {
	match key.drop_meta() {
		Node::Symbol(name) | Node::Text(name) => Some(name.clone()),
		Node::Char(letter) => Some(letter.to_string()), // `{"A": 1}`: a quoted one-letter key
		_ => None,
	}
}

/// Variables that are only ever assigned an object literal, with that literal (`None` for a variable assigned anything
/// else too), and the variables assigned a plain value (`plain`: a number, text or character, or a list of them)
#[derive(Default)]
struct AssignedObjects {
	objects: HashMap<String, Option<Node>>,
	plain: HashSet<String>,
	/// variables also bound to what may hold an object, so not plain: a loop's `for item in basket`, `p = people#1`
	rebound: HashSet<String>,
}

fn collect_assigned_objects(node: &Node, assigned: &mut AssignedObjects) {
	match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, value) => {
			if let Node::Symbol(name) = target.drop_meta() {
				if is_plain_literal(value) {
					assigned.plain.insert(name.clone());
				}
				if may_hold_object(value) {
					assigned.rebound.insert(name.clone());
				}
				let copied = match value.drop_meta() {
					Node::Symbol(other) => assigned.objects.get(other).cloned().flatten(), // `q=p` is the same object
					_ => None,
				};
				let literal = copied.or_else(|| object_entries(value).map(|_| value.as_ref().clone()));
				let both_objects = literal.is_some() && assigned.objects.get(name).is_none_or(|earlier| earlier.is_some());
				assigned.objects.insert(name.clone(), if both_objects { literal } else { None });
			}
			collect_assigned_objects(value, assigned);
		}
		Node::Key(left, _, right) => {
			collect_assigned_objects(left, assigned);
			collect_assigned_objects(right, assigned);
		}
		Node::List(items, _, _) => {
			if let Some(variable) = loop_variable(items) {
				assigned.rebound.insert(variable);
			}
			items.iter().for_each(|item| collect_assigned_objects(item, assigned))
		}
		_ => {}
	}
}

/// An element, a field, a call's result or an object literal
fn may_hold_object(value: &Node) -> bool {
	match value.drop_meta() {
		Node::Key(_, Op::Hash | Op::Dot, _) => true,
		Node::List(items, Bracket::Round, Separator::None) => matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))),
		other => object_entries(other).is_some(),
	}
}

/// The variable `item` of a loop `for item in basket {…}`, `for each item in basket {…}`
fn loop_variable(words: &[Node]) -> Option<String> {
	let words: Vec<String> = words.iter().take(4).map(|word| word.drop_meta().name()).collect();
	match words.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
		["for", "each", variable, "in"] | ["for", variable, "in", ..] => Some(variable.to_string()),
		_ => None,
	}
}

/// A number, text or character literal, or a list literal of them
fn is_plain_literal(value: &Node) -> bool {
	match value.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False => true,
		Node::List(items, Bracket::Square, _) => !items.is_empty() && items.iter().all(is_plain_literal),
		_ => false,
	}
}

/// `person: {name: "Alice"}` among the statements of a program that reads `person` later binds it like
/// `person = {…}`: `person.name` is then a field of a variable (a data key nothing reads stays data)
fn bind_read_data_objects(program: Node) -> Node {
	let Node::List(items, bracket @ (Bracket::None | Bracket::Curly), separator) = program else { return program };
	let mut read = HashSet::new();
	for item in &items {
		item.visit(&mut |part| {
			if let Node::Key(receiver, Op::Dot | Op::Hash, _) = part {
				if let Node::Symbol(name) = receiver.drop_meta() {
					read.insert(name.clone());
				}
			}
		});
	}
	let items = items.into_iter().map(|item| match item.drop_meta() {
		Node::Key(name, Op::Colon, value) if matches!(name.drop_meta(), Node::Symbol(symbol) if read.contains(symbol)) && object_entries(value).is_some() => {
			Node::Key(name.clone(), Op::Assign, value.clone())
		}
		_ => item,
	}).collect();
	Node::List(items, bracket, separator)
}

/// The names the program defines, as variables or functions: an alias word never shadows one (P142)
pub(crate) fn defined_names(node: &Node) -> HashSet<String> {
	let mut defined = HashSet::new();
	collect_assigned_names(node, &mut defined);
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, node);
	defined.extend(context.user_functions.into_keys());
	defined
}

pub(crate) fn collect_assigned_names(node: &Node, names: &mut HashSet<String>) {
	match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, value) => {
			if let Node::Symbol(name) = target.drop_meta() {
				names.insert(name.clone());
			}
			collect_assigned_names(value, names);
		}
		Node::Key(left, _, right) => {
			collect_assigned_names(left, names);
			collect_assigned_names(right, names);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| collect_assigned_names(item, names)),
		_ => {}
	}
}

struct Lowering {
	context: Context,
	shadowed: HashSet<String>,
	objects: HashMap<String, Node>,
	/// Variables assigned plain values (AssignedObjects::plain)
	plain: HashSet<String>,
	/// Values of declared types (`p:person`, `first = (sort people)#1`) read their fields like object literals
	instances: crate::traits::InstanceTypes,
	/// The untyped parameters of the definitions being lowered: any of them may hold an object, so `p.width` reads a field
	parameters: RefCell<Vec<String>>,
	/// The functions being defined, innermost last, each with its parameters: `start(l) := l.start` reads the field
	defining: RefCell<Vec<(String, Vec<String>)>>,
	/// Variables assigned the result of a user function call (call_results)
	call_results: HashSet<String>,
	temporaries: Cell<usize>,
}

impl Lowering {
	fn expand(&self, node: Node) -> Node {
		match node {
			Node::List(items, Bracket::Round, _) if items.len() == 3 && is_marker(&items[0], TRY_MARKER) => {
				self.lower_try(self.expand(items[1].clone()), self.expand(items[2].clone()))
			}
			// `catch e { … }`: the fallback reads the caught Error as e (P67)
			Node::List(items, Bracket::Round, _) if items.len() == 4 && is_marker(&items[0], TRY_MARKER) => {
				self.lower_try_binding(self.expand(items[1].clone()), self.expand(items[2].clone()), &items[3])
			}
			Node::List(items, Bracket::Round, _) if items.len() == 3 && is_marker(&items[0], ASSERT_MARKER) => {
				self.lower_assert(self.expand(items[1].clone()), self.expand(items[2].clone()))
			}
			// `def f(m) { m.a }`: the body sees the parameters, as `f(m) := m.a` does
			Node::List(items, bracket, separator) if keyword_definition_parameters(&items).is_some() => {
				let parameters = keyword_definition_parameters(&items).expect("guarded");
				let name = keyword_definition_name(&items);
				let items = self.within_function(name, parameters, || items.into_iter().map(|item| self.expand(item)).collect());
				Node::List(items, bracket, separator)
			}
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.expand(item)).collect();
				if let Some(walk) = self.for_over_map(&items) {
					return walk;
				}
				self.word_call(&items, &bracket, &separator).unwrap_or(Node::List(items, bracket, separator))
			}
			// `p.x++` (parsed as `p.(x++)` or `(p.x)++`) is `p.x += 1`, an update of the field
			Node::Key(left, Op::Dot, right) if field_step(&right).is_some() => {
				let (field, op) = field_step(&right).expect("guarded");
				self.expand(stepped(Node::Key(left, Op::Dot, Box::new(field)), op))
			}
			// `xs#1++`, `bags#1.n++`: the whole place steps
			Node::Key(left, op @ (Op::Inc | Op::Dec), _) if matches!(left.drop_meta(), Node::Key(_, Op::Dot | Op::Hash, _)) => self.expand(stepped(*left, op)),
			// `bags#1.n += 4`, `p.b.n += 2`, `xs#1#2 += 1`: `place = place + 4`, stored back into the place
			Node::Key(left, op, right) if op.is_compound_assign() && is_nested_place(&left) => {
				let combined = Node::Key(left.clone(), op.base_op(), right);
				let left = self.expand(*left);
				let combined = self.expand(combined);
				self.stored(&left, combined.clone()).unwrap_or(Node::Key(Box::new(left), Op::Assign, Box::new(combined)))
			}
			Node::Key(left, Op::Dot, right) => {
				let (left, right) = (self.expand(*left), self.expand_method(*right));
				self.method_call(&left, &right).unwrap_or(Node::Key(Box::new(left), Op::Dot, Box::new(right)))
			}
			Node::Key(left, Op::Assign, right) => {
				if let Some(chained) = self.chained_field_assignment(&left, &right) {
					return chained;
				}
				let left = self.expand(*left);
				let right = self.in_definition(&left, *right);
				self.field_assignment(&left, &right).unwrap_or(Node::Key(Box::new(left), Op::Assign, Box::new(right)))
			}
			Node::Key(left, Op::SafeDot, right) => {
				let (receiver, word) = (self.expand(*left), *right);
				self.safe_lookup(receiver, word)
			}
			Node::Key(left, op @ (Op::Similar | Op::Rough), right) => self.lower_similar(op, self.expand(*left), self.expand(*right)),
			Node::Key(left, Op::Coalesce, right) => self.lower_coalesce(self.expand(*left), self.expand(*right)),
			Node::Key(left, Op::Define, right) => {
				let left = self.expand(*left);
				let right = self.in_definition(&left, *right);
				Node::Key(Box::new(left), Op::Define, Box::new(right))
			}
			// P66: `x / 0.0`, a float division by a written zero, is IEEE's ∞ (an exact division by zero is divide_by_zero)
			Node::Key(left, Op::Div, right) if is_float_zero(&right) => {
				let as_float = |operand: Node| Node::Key(Box::new(operand), Op::As, Box::new(Node::Symbol(FLOAT_WORD.to_string())));
				Node::Key(Box::new(as_float(self.expand(*left))), Op::Div, Box::new(as_float(*right)))
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.expand(*left)), op, Box::new(self.expand(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.expand(*node)), data },
			other => other,
		}
	}

	/// The right side of `head = right` or `head := right`, with the untyped parameters of a definition head in scope
	fn in_definition(&self, head: &Node, right: Node) -> Node {
		let parameters = crate::traits::untyped_parameters(head);
		let name = match head.drop_meta() {
			Node::List(items, Bracket::Round, _) => items.first().map(Node::name),
			_ => None,
		};
		self.within_function(name, parameters, || self.expand(self.body_statement(head, right)))
	}

	/// `lower` with the parameters of a definition in scope, and its function's name when it has one
	fn within_function<T>(&self, name: Option<String>, parameters: Vec<String>, lower: impl FnOnce() -> T) -> T {
		let scope = self.parameters.borrow().len();
		self.parameters.borrow_mut().extend(parameters.iter().cloned());
		let named = name.map(|name| self.defining.borrow_mut().push((name, parameters))).is_some();
		let lowered = lower();
		self.parameters.borrow_mut().truncate(scope);
		if named {
			self.defining.borrow_mut().pop();
		}
		lowered
	}

	/// `p.name` inside the function `name(p)`: a call would recurse with the same argument forever, so it reads the field
	fn reads_own_parameter_field(&self, receiver: &Node, name: &str) -> bool {
		let Node::Symbol(receiver) = receiver.drop_meta() else { return false };
		self.defining.borrow().last().is_some_and(|(function, parameters)| function == name && parameters.contains(receiver))
	}

	/// `def total(xs){sum xs}`: the body block of a function that is one braceless call is that statement, not the data
	/// `{sum xs}`; an undefined word in it (`{square x}`) is then reported as one
	fn body_statement(&self, head: &Node, right: Node) -> Node {
		let defines_function = matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))));
		match right {
			Node::List(items, Bracket::Curly, Separator::Space) if defines_function && items.len() > 1 && matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => {
				Node::List(vec![Node::List(items, Bracket::None, Separator::Space)], Bracket::Curly, Separator::Semicolon)
			}
			other => other,
		}
	}

	fn is_parameter(&self, node: &Node) -> bool {
		matches!(node.drop_meta(), Node::Symbol(name) if self.parameters.borrow().contains(name))
	}

	/// `a ≈ b`, `a ~ b`, `a circa b`: compared within the relative tolerance; a side that is more than a plain value or
	/// arithmetic is computed once into a temporary
	/// `a ≈ b`: the emitter's values_similar(a, b, tolerance), numbers inline, any other values field by field
	fn lower_similar(&self, op: Op, left: Node, right: Node) -> Node {
		let (_, function, variable, default) = SIMILARITY_LEVELS.iter().find(|(level, ..)| *level == op).expect("a similarity operator");
		let tolerance = if self.shadowed.contains(*variable) { variable } else { default };
		let call = vec![Node::Symbol(function.to_string()), left, right, crate::warp_parser::parse(tolerance)];
		Node::List(call, Bracket::Round, Separator::None)
	}

	/// `a ?? b`: a unless it is ø, then b (a is computed once)
	fn lower_coalesce(&self, left: Node, right: Node) -> Node {
		let mut bindings = vec![];
		let left = self.bound_once(left, &mut bindings);
		let (l, r) = (LEFT_OPERAND_PLACEHOLDER, RIGHT_OPERAND_PLACEHOLDER);
		let choice = self.instantiate_template(&format!("if {l} == ø then {r} else {l}"), &[(l, &left), (r, &right)]);
		crate::min_max::with_bindings(bindings, choice)
	}

	fn bound_once(&self, operand: Node, bindings: &mut Vec<Node>) -> Node {
		if crate::min_max::is_plain(&operand) {
			return operand;
		}
		let temporary = Node::Symbol(format!("{OPERAND_TEMPORARY}_{}", self.next_temporary()));
		bindings.push(Node::Key(Box::new(temporary.clone()), Op::Assign, Box::new(operand)));
		temporary
	}

	/// The word after a dot, `word` or `word(arguments)`: only the arguments are expanded, the word is not a call of its own
	fn expand_method(&self, method: Node) -> Node {
		match method {
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.expand_method(*node)), data },
			Node::List(items, bracket, separator) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => {
				let mut items = items.into_iter();
				let word = items.next().expect("guarded");
				Node::List([vec![word], items.map(|argument| self.expand(argument)).collect()].concat(), bracket, separator)
			}
			other => self.expand(other),
		}
	}

	fn library_word(&self, name: &str) -> Option<&'static str> {
		canonical_word(name).filter(|word| !self.shadowed.contains(name) && !self.shadowed.contains(*word))
	}

	/// The library word a call of `name` with that many arguments (receiver included) means
	fn library_word_for(&self, name: &str, argument_count: usize) -> Option<&'static str> {
		self.arity_variant(name, argument_count).or_else(|| self.library_word(name))
	}

	/// The word `name` means with that many arguments, unless the program defines `name`
	fn arity_variant(&self, name: &str, argument_count: usize) -> Option<&'static str> {
		let variant = ARITY_VARIANTS.iter().find(|(word, count, _)| *word == name && *count == argument_count).map(|(_, _, variant)| *variant);
		variant.filter(|_| !self.shadowed.contains(name))
	}

	/// `word(x, args)` and `word x args`
	fn word_call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		// `x = sum of xs`, `f() := sum of xs`: an assigned value nests the words after the first, `[sum, [of, xs]]`
		if let [head, Node::List(rest, Bracket::None, Separator::Space)] = items {
			if rest.first().is_some_and(is_of) {
				return self.word_call(&[vec![head.clone()], rest.clone()].concat(), bracket, separator);
			}
		}
		if let Some(lookup) = self.of_lookup(items) {
			return Some(lookup);
		}
		if let Some(membership) = self.membership(items).or_else(|| self.containment(items)) {
			return Some(membership);
		}
		if let Some(raised) = raise_call(items) {
			return Some(raised);
		}
		if let Err(error) = self.ask_braceless_argument(items, bracket, separator) {
			return Some(error);
		}
		let head = match items.first()?.drop_meta() {
			Node::Symbol(name) => name,
			_ => return None,
		};
		// `log(x, base)` is ln(x)/ln(base): libm's log takes one argument (unless the program defines its own log)
		if let (LOG_WORD, [_, value, base]) = (head.as_str(), items) {
			if self.shadowed.contains(LOG_WORD) {
				return None;
			}
			if call_name(items, bracket, separator).is_some() {
				let log_of = |argument: &Node| Node::List(vec![items[0].clone(), argument.clone()], Bracket::Round, Separator::None);
				return Some(Node::Key(Box::new(log_of(value)), Op::Div, Box::new(log_of(base))));
			}
		}
		// `first 3 of xs` is `first_items(xs, 3)`
		if let [_, argument, of, object] = items {
			if let (true, Some(word)) = (is_of(of), self.arity_variant(head, 2)) {
				return Some(self.call(word, &items[0], vec![self.expand(object.clone()), self.expand(argument.clone())], false));
			}
		}
		// `first of xs` has one argument, as `first sorted xs`: a library word after it is its argument's prefix
		let prefixes_its_argument = *bracket == Bracket::None && items.get(1).is_some_and(|next| self.library_word(&next.drop_meta().name()).is_some());
		let argument_count = if items.get(1).is_some_and(is_of) || prefixes_its_argument { 1 } else { items.len() - 1 };
		let word = self.library_word_for(head, argument_count)?;
		// `sorted "listen" == sorted "silent"`: the word takes the operand of the comparison, as a defined function of `it` does
		if let ([_, argument], Bracket::None, Separator::Space) = (items, bracket, separator) {
			if is_comparison(argument) {
				return Some(with_first_comparand(argument.clone(), &|operand| self.call(word, &items[0], vec![operand], true)));
			}
		}
		if let [_, of, rest @ ..] = items {
			if is_of(of) && !rest.is_empty() {
				// `first of xs` is `first(xs)`
				let argument = match rest {
					[single] => single.clone(),
					_ => Node::List(rest.to_vec(), Bracket::None, Separator::Space),
				};
				return Some(self.call(word, &items[0], vec![argument], false));
			}
		}
		let is_call = call_name(items, bracket, separator).is_some();
		// `(sort xs)` is `sort xs` in parentheses
		let is_prefix = matches!(bracket, Bracket::None | Bracket::Round) && *separator == Separator::Space && items.len() > 1;
		// `[sum xs]`: a word with exactly its arguments in a list is its one computed element
		// (a bare word that names no variable keeps the list data: `[first last]`)
		let is_value = |argument: &Node| !matches!(argument.drop_meta(), Node::Symbol(name) if !self.shadowed.contains(name));
		let is_element = *bracket == Bracket::Square && *separator == Separator::Space && items.len() == arity(word) + 1 && items[1..].iter().all(is_value);
		if is_element && !is_call {
			let element = self.call(word, &items[0], items[1..].to_vec(), false);
			return Some(Node::List(vec![element], Bracket::Square, Separator::None));
		}
		if !is_call && !is_prefix {
			return None;
		}
		Some(self.call(word, &items[0], items[1..].to_vec(), is_prefix && !is_call))
	}

	/// `for k in m` walks the keys of a map, as in Python; `for k, v in m` and `for (k, v) in m` bind each value too:
	/// `for k in map_keys(m) {v = m[k]; …}`
	fn for_over_map(&self, items: &[Node]) -> Option<Node> {
		let [keyword, names, in_word, rest @ ..] = items else { return None };
		if !is_marker(keyword, FOR_WORD) || !is_marker(in_word, IN_WORD) {
			return None;
		}
		let (map, body) = match rest {
			[map, body] => (map.clone(), body.clone()),
			[colon] => match colon.drop_meta() {
				Node::Key(map, Op::Colon, body) => (map.as_ref().clone(), body.as_ref().clone()),
				_ => return None,
			},
			_ => return None,
		};
		self.object_literal(&map)?;
		let (key, value) = match names.drop_meta() {
			Node::Symbol(_) => (names.clone(), None),
			Node::List(parts, Bracket::Round | Bracket::None, _) => match parts.as_slice() {
				[key, value] if [key, value].iter().all(|name| matches!(name.drop_meta(), Node::Symbol(_))) => (key.clone(), Some(value.clone())),
				_ => return None,
			},
			_ => return None,
		};
		let body = match value {
			None => body,
			Some(value) => {
				let lookup = Node::Key(Box::new(value), Op::Assign, Box::new(crate::warp_parser::subscript(map.clone(), key.clone())));
				Node::List([vec![lookup], crate::for_loop::block_items(&body)].concat(), Bracket::Curly, Separator::Semicolon)
			}
		};
		let keys = self.call(MAP_KEYS, keyword, vec![map], false);
		Some(Node::List(vec![keyword.clone(), key, in_word.clone(), keys, body], Bracket::None, Separator::Space))
	}

	/// `square 3 + square(4)`, a braceless call of a user function whose argument holds an operator, reads as
	/// square(3 + square(4)); P68 (user): the reading stays, with a got-it warning naming both readings
	fn ask_braceless_argument(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Result<(), Node> {
		let [function, argument] = items else { return Ok(()) };
		let Node::Symbol(name) = function.drop_meta() else { return Ok(()) };
		let Node::Key(left, op, right) = argument.drop_meta() else { return Ok(()) };
		if *bracket != Bracket::None || *separator != Separator::Space || !self.context.user_functions.contains_key(name)
			|| !op.is_arithmetic() || matches!(left.drop_meta(), Node::Empty) {
			return Ok(());
		}
		let (left, right) = (call_text(left), call_text(right));
		let whole = format!("{name}({left} {op} {right})");
		let first = format!("{name}({left}) {op} {right}");
		let question = crate::diagnostic::Ask::new(BRACELESS_ARGUMENT_TOPIC, format!("`{name} {left} {op} {right}` is {whole}, not {first}"),
			vec![crate::diagnostic::reading("the whole expression", &whole), crate::diagnostic::reading("only the first operand", &first)],
			crate::diagnostic::Fallback::Warning).written(&format!("{name} {left} {op} {right}")).at_node(argument);
		crate::diagnostic::ask(&question).map(|_| ())
	}

	/// `x in xs`: whether a map has the key x or a list the element x; `byte in t` and `"äb" in bytes` count text units
	fn membership(&self, items: &[Node]) -> Option<Node> {
		let [element, in_word, collection] = items else { return None };
		let names_unit = |side: &Node| match side.drop_meta() {
			Node::Key(_, Op::Hash, unit) => crate::analyzer::text_unit(&unit.name()).is_some(),
			other => crate::analyzer::text_unit(&other.name()).is_some(),
		};
		if !is_marker(in_word, IN_WORD) || names_unit(element) || names_unit(collection) {
			return None;
		}
		Some(self.call(COLLECTION_POSITION, in_word, vec![collection.clone(), element.clone()], false))
	}

	/// `xs contains x`, `xs has x`, `xs includes x`: the method `xs.contains(x)` written between its two values, unless
	/// the program defines the word
	fn containment(&self, items: &[Node]) -> Option<Node> {
		let [collection, word, element] = items else { return None };
		let Node::Symbol(name) = word.drop_meta() else { return None };
		let canonical = self.library_word_for(name, 2).filter(|canonical| *canonical == COLLECTION_CONTAINS)?;
		Some(self.call(canonical, word, vec![collection.clone(), element.clone()], false))
	}

	/// `x.word` and `x.word(args)`; an unknown word on a value is an error
	fn method_call(&self, receiver: &Node, method: &Node) -> Option<Node> {
		// `pair.0` is the first item of a tuple or list, counted from 0 like `pair[0]`
		if let Node::Number(crate::extensions::numbers::Number::Int(_)) = method.drop_meta() {
			return Some(crate::warp_parser::subscript(receiver.clone(), method.clone()));
		}
		let (word_node, arguments) = match method.drop_meta() {
			Node::Symbol(_) => (method, vec![]),
			Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => (&items[0], items[1..].to_vec()),
			_ => return None,
		};
		let Node::Symbol(name) = word_node.drop_meta() else { return None };
		let has_arguments = !arguments.is_empty();
		let literal = self.object_literal(receiver);
		let is_field = literal.as_ref().and_then(object_entries).is_some_and(|entries| entries.iter().any(|(field, _)| field == name));
		if is_field && !has_arguments {
			return Some(field_lookup(receiver, name, word_node));
		}
		// `x.chars` stays the count of characters; `chars x`, `chars(x)` and the call `x.chars()` are the list
		let is_called = matches!(method.drop_meta(), Node::List(..));
		if let Some(word) = self.library_word_for(name, arguments.len() + 1).filter(|_| is_called || counting_method(name, &self.context).is_none()) {
			return Some(self.call(word, word_node, [vec![receiver.clone()], arguments].concat(), false));
		}
		// `x.square` and `x.add(y)` call the user function with the receiver as first argument; `l.start` of an object
		// with a field start reads the field, even beside a function start (`start(l) := l.start`)
		let reads_declared_field = !is_called && self.instances.is_declared_field(name) && (self.is_parameter(receiver) || self.instances.shape(receiver).is_some());
		let reads_declared_field = reads_declared_field || (!is_called && self.reads_own_parameter_field(receiver, name));
		if reads_declared_field {
			return Some(field_lookup(receiver, name, word_node));
		}
		if self.context.user_functions.get(name).is_some_and(|function| !function.params.is_empty()) {
			let call = [vec![word_node.clone(), receiver.clone()], arguments].concat();
			return Some(Node::List(call, Bracket::Round, Separator::None));
		}
		// `s.trim()`, `s.byte_at(2)`: a text builtin with the receiver as first argument
		if is_called && crate::wasm_emitter::text_builtins::text_builtin_kind(name, arguments.len() + 1).is_some() {
			let call = [vec![word_node.clone(), receiver.clone()], arguments].concat();
			return Some(Node::List(call, Bracket::Round, Separator::None));
		}
		// `x.string`, `t.int`, `x.float()`: the conversion `string(x)`, as written in call form
		// (not `xs.number`, a count, nor `e.text`, a common field)
		if CONVERSION_METHODS.contains(&name.as_str()) && arguments.is_empty() && !self.instances.is_declared_field(name) {
			return Some(Node::List(vec![word_node.clone(), receiver.clone()], Bracket::Round, Separator::None));
		}
		let is_known = counting_method(name, &self.context).is_some() || is_list_mutating_method(name) || self.context.user_functions.contains_key(name);
		let is_call_result = matches!(receiver.drop_meta(), Node::Symbol(variable) if self.call_results.contains(variable));
		let is_object = literal.is_some() || is_field_lookup(receiver) || self.is_parameter(receiver) || is_call_result || self.instances.is_declared_field(name);
		if is_object && !is_known && !has_arguments {
			return Some(field_lookup(receiver, name, word_node));
		}
		// a variable of unknown value (`o` of `for o in people.filter(…)`), or an element of one (`xs[1]`), may hold an
		// object: its field is looked up at run time
		let variable = match receiver.drop_meta() {
			Node::Key(list, Op::Hash, _) => list.drop_meta(),
			other => other,
		};
		let may_hold_object = match variable {
			Node::Symbol(variable) => !self.plain.contains(variable) && !self.context.user_functions.contains_key(variable),
			// the value of a call: `f().name`, `lib.stats_of(t).words` (foreign_call), or a parenthesized object `({a:1}).a`
			Node::List(items, Bracket::Round, Separator::None) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => true,
			Node::List(items, Bracket::Round, _) if items.len() == 1 => matches!(items[0].drop_meta(), Node::List(_, Bracket::Curly, _)),
			// an element of a computed list: `users.filter(…)#1.name`
			_ => matches!(receiver.drop_meta(), Node::Key(_, Op::Hash, _)) && !is_plain_literal(variable),
		};
		if may_hold_object && !is_known && !has_arguments {
			return Some(field_lookup(receiver, name, word_node));
		}
		// `x.type`, `x.type()`: `type(x)`, once no field of that name can be meant
		if name == crate::type_tests::TYPE_WORD && !has_arguments {
			return Some(Node::List(vec![word_node.clone(), receiver.clone()], Bracket::Round, Separator::None));
		}
		let is_value = matches!(receiver.drop_meta(), Node::Symbol(_) | Node::Text(_) | Node::Char(_) | Node::Number(_) | Node::List(_, Bracket::Square, _));
		let call = Node::Key(Box::new(receiver.clone()), Op::Dot, Box::new(method.clone()));
		(!is_known && is_value).then(|| self.context.undefined_function_diagnostic(&call, name).into_error())
	}

	/// `object.name = value` is `object = field_with(object, "name", value)`; a nested path updates the objects on the way
	/// (`p.b.c = 4` is `p = field_with(p, "b", field_with(p.b, "c", 4))`), an element on the way is stored back
	/// (`bags#1.n = 5` is `bags#1 = field_with(bags#1, "n", 5)`)
	fn field_assignment(&self, target: &Node, value: &Node) -> Option<Node> {
		let Node::Key(_, Op::Hash, index) = target.drop_meta() else { return None };
		subscript_field(index)?;
		self.stored(target, value.clone())
	}

	/// `a = p.x = 5` is `p.x = 5; a = p.x`: the value of a field assignment is the field's new value, not the object
	/// field_with gives (card chain-field)
	fn chained_field_assignment(&self, outer: &Node, value: &Node) -> Option<Node> {
		let Node::Key(inner, Op::Assign, _) = value.drop_meta() else { return None };
		let Node::Key(_, Op::Hash, index) = self.expand(inner.as_ref().clone()).drop_meta().clone() else { return None };
		subscript_field(&index)?;
		let read_back = Node::Key(Box::new(outer.clone()), Op::Assign, inner.clone());
		Some(self.expand(Node::List(vec![value.clone(), read_back], Bracket::None, Separator::Semicolon)))
	}

	/// `place = value` for any place: a variable, an element `xs#i` of one (as written), a field (field_with on its
	/// object, stored back), an element of anything else through a temporary (`p.items#1 = v` is
	/// `place·0 = p.items; place·0#1 = v; p.items = place·0`)
	fn stored(&self, place: &Node, value: Node) -> Option<Node> {
		let assigned = |place: &Node, value: Node| Node::Key(Box::new(place.clone()), Op::Assign, Box::new(value));
		let Node::Key(base, Op::Hash, index) = place.drop_meta() else {
			return matches!(place.drop_meta(), Node::Symbol(_)).then(|| assigned(place, value));
		};
		if let Some(name) = subscript_field(index) {
			let updated = Node::List(vec![Node::Symbol(FIELD_WITH.to_string()), base.as_ref().clone(), Node::Text(name), value], Bracket::Round, Separator::None);
			return self.stored(base, updated);
		}
		if matches!(base.drop_meta(), Node::Symbol(_)) {
			return Some(assigned(place, value));
		}
		let held = Node::Symbol(format!("{PLACE_TEMPORARY}{}", self.next_temporary()));
		let element = Node::Key(Box::new(held.clone()), Op::Hash, index.clone());
		let statements = vec![assigned(&held, base.as_ref().clone()), assigned(&element, value), self.stored(base, held)?];
		Some(Node::List(statements, Bracket::None, Separator::Semicolon))
	}

	fn next_temporary(&self) -> usize {
		let number = self.temporaries.get();
		self.temporaries.set(number + 1);
		number
	}

	/// `receiver?.name`: ø when the receiver is ø, else the field
	fn safe_lookup(&self, receiver: Node, word: Node) -> Node {
		let Node::Symbol(name) = word.drop_meta() else {
			return Diagnostic::at(&word, "?. needs a field name after it").into_error();
		};
		let temporary = format!("safe_tmp_{}", self.next_temporary());
		let program = parse(&format!("({temporary}={RECEIVER_PLACEHOLDER}; if {temporary} == ø then ø else {LOOKUP_PLACEHOLDER})"));
		let lookup = field_lookup(&Node::Symbol(temporary), name, &word);
		substitute(substitute(program, RECEIVER_PLACEHOLDER, &receiver), LOOKUP_PLACEHOLDER, &lookup)
	}

	/// The object literal a node stands for: the literal itself, a variable only ever assigned one, or a field of such an object
	fn object_literal(&self, node: &Node) -> Option<Node> {
		self.written_object(node).or_else(|| self.instances.fields_template(node))
	}

	fn written_object(&self, node: &Node) -> Option<Node> {
		match node.drop_meta() {
			Node::Symbol(name) => self.objects.get(name).cloned(),
			Node::Key(base, Op::Hash, index) => {
				let field = field_name(crate::warp_parser::subscript_key(index)?)?;
				let (_, value) = object_entries(&self.object_literal(base)?)?.into_iter().find(|(name, _)| *name == field)?;
				object_entries(&value).map(|_| value)
			}
			other => object_entries(other).map(|_| other.clone()),
		}
	}

	/// `word of object`, `c of b of p`: the lookup of a field, when the object is one; `None` for anything else
	fn of_lookup(&self, items: &[Node]) -> Option<Node> {
		let [word_node, of, rest @ ..] = items else { return None };
		let Node::Symbol(name) = word_node.drop_meta() else { return None };
		if !is_of(of) || rest.is_empty() {
			return None;
		}
		let object = match rest {
			[single] => single.clone(),
			[_, next_of, ..] if is_of(next_of) => self.of_lookup(rest)?,
			_ => return None,
		};
		let literal = self.object_literal(&object);
		let is_field = literal.as_ref().and_then(object_entries).is_some_and(|entries| entries.iter().any(|(field, _)| field == name));
		let is_object = literal.is_some() || is_field_lookup(&object);
		let is_other_word = self.library_word(name).is_some() || counting_method(name, &self.context).is_some();
		(is_object && (is_field || !is_other_word)).then(|| field_lookup(&object, name, word_node))
	}

	/// The call of `word` with the receiver and its arguments; arguments are the items after the word for the prefix form
	fn call(&self, word: &'static str, head: &Node, arguments: Vec<Node>, is_prefix: bool) -> Node {
		let mut arguments = if is_prefix { self.merged_prefix_arguments(word, arguments) } else { arguments };
		let wanted = arity(word);
		let optional = OPTIONAL_ARGUMENTS.iter().find(|(name, _)| *name == word).map_or(0, |(_, optional)| *optional);
		if arguments.len() < wanted && arguments.len() + optional >= wanted {
			arguments.resize(wanted, left_out_argument(word));
		}
		// objects are references (P200): `x.copy()` is a new deep copy, `x.copy(shallow: true)` a shallow one (P205)
		if let (COPY, [receiver], shallow) | (COPY, [receiver, shallow], _) = (word, arguments.as_slice(), &Node::False) {
			return copy_call(receiver.clone(), shallow_flag(shallow));
		}
		// a text builtin's own arity check names its values (`trim takes 1 value, got 2`)
		let checks_itself = crate::wasm_emitter::text_builtins::is_text_builtin(word);
		if arguments.len() != wanted && !checks_itself {
			let plural = if wanted == 1 { "" } else { "s" };
			let call = Node::List([vec![head.clone()], arguments.clone()].concat(), Bracket::Round, Separator::None);
			return Diagnostic::at(&call, format!("{word} takes {wanted} argument{plural}, got {}", arguments.len())).into_error();
		}
		match EXPANDED_WORDS.iter().find(|(name, _, _)| *name == word) {
			Some((_, _, template)) if word == SUM => dispatched_sum(self.expanded(template, arguments)),
			Some((_, _, template)) => self.expanded(template, arguments),
			// a prelude word calls its definition from lib/prelude.warp (modules::prelude_name)
			None if let Some(qualified) = crate::modules::prelude_name(word) => Node::List([vec![Node::Symbol(qualified)], arguments].concat(), Bracket::Round, Separator::None),
			None => {
				let name = if matches!(head.drop_meta(), Node::Symbol(written) if written == word) { head.clone() } else { Node::Symbol(word.to_string()) };
				Node::List([vec![name], arguments].concat(), Bracket::Round, Separator::None)
			}
		}
	}

	/// The arguments after the first wanted-1 of a braceless call as one phrase; a phrase heading with a library word is
	/// its call: `first sort xs` is `first(sort(xs))`
	fn merged_prefix_arguments(&self, word: &str, arguments: Vec<Node>) -> Vec<Node> {
		let wanted = arity(word);
		if arguments.len() <= wanted {
			return arguments;
		}
		let (kept, rest) = arguments.split_at(wanted - 1);
		let heads_with_word = matches!(rest[0].drop_meta(), Node::Symbol(name) if self.library_word(name).is_some());
		let phrase = heads_with_word.then(|| self.word_call(rest, &Bracket::None, &Separator::Space)).flatten();
		let phrase = phrase.unwrap_or_else(|| Node::List(rest.to_vec(), Bracket::None, Separator::Space));
		[kept.to_vec(), vec![phrase]].concat()
	}

	/// A source template with `hidden` variables made unique, its placeholders replaced by the given nodes
	fn instantiate_template(&self, template: &str, replacements: &[(&str, &Node)]) -> Node {
		let number = self.temporaries.get();
		self.temporaries.set(number + 1);
		let mut program = parse(&template.replace(TRY_TEMPORARY, &format!("{TRY_TEMPORARY}_{number}")));
		for (placeholder, replacement) in replacements {
			program = substitute(program, placeholder, replacement);
		}
		program
	}

	/// A list or text written in place (`[1 2]#3`, `"ab"#5`): `count` of it cannot fail
	fn is_known_list(&self, list: &Node) -> bool {
		matches!(list.drop_meta(), Node::List(_, Bracket::Square, _) | Node::Text(_))
	}

	/// `try X else Y`: X, or Y when X fails. Errors are values: an Error result is replaced. An index out of range and a
	/// division or modulo by zero directly under `try` are checked before they happen; a runtime error deeper inside X
	/// (an index in a sum, a called function) is caught as a wasm exception (`ran_without_error`, wasm_emitter/try_guard.rs).
	fn lower_try(&self, guarded: Node, fallback: Node) -> Node {
		let (value, fallback_placeholder) = (TRY_VALUE_PLACEHOLDER, TRY_FALLBACK_PLACEHOLDER);
		match guarded.drop_meta() {
			Node::Key(target, Op::Assign, assigned) if matches!(target.drop_meta(), Node::Symbol(_)) => {
				Node::Key(target.clone(), Op::Assign, Box::new(self.lower_try(assigned.as_ref().clone(), fallback)))
			}
			// a list written in place: the index is checked before it happens; any other value
			// may be no list at all (`n#1`, `xs#2#1`), which the exception path catches as not_a_list
			Node::Key(list, Op::Hash, index) if !matches!(list.drop_meta(), Node::Empty) && self.is_known_list(list) => self.instantiate_template(
				&format!("(try_tmp_list={LIST_PLACEHOLDER}; try_tmp_index={INDEX_PLACEHOLDER}; if try_tmp_index >= 1 and try_tmp_index <= count(try_tmp_list) {{try_tmp_list#try_tmp_index}} else {{{fallback_placeholder}}})"),
				&[(LIST_PLACEHOLDER, list), (INDEX_PLACEHOLDER, index), (fallback_placeholder, &fallback)],
			),
			// the divisor checked first; any other error of the division (a text divided) is caught as below
			Node::Key(dividend, op @ (Op::Div | Op::Mod | Op::Rem), divisor) => self.instantiate_template(
				&format!("(try_tmp_divisor={DIVISOR_PLACEHOLDER}; if try_tmp_divisor==0 {{{fallback_placeholder}}} else {{\
					try_tmp_finished={RAN_WITHOUT_ERROR}({{try_tmp_value={DIVIDEND_PLACEHOLDER} {op} try_tmp_divisor}}); \
					if try_tmp_finished and not is_error(try_tmp_value) {{try_tmp_value}} else {{{fallback_placeholder}}}}})"),
				&[(DIVISOR_PLACEHOLDER, divisor), (DIVIDEND_PLACEHOLDER, dividend), (fallback_placeholder, &fallback)],
			),
			_ => self.instantiate_template(
				// the guarded assignment comes first, so the kind of try_tmp_value is known where it is read
				&format!("(try_tmp_finished={RAN_WITHOUT_ERROR}({{try_tmp_value={value}}}); if try_tmp_finished and not is_error(try_tmp_value) {{try_tmp_value}} else {{{fallback_placeholder}}})"),
				&[(value, &guarded), (fallback_placeholder, &fallback)],
			),
		}
	}

	/// `try X catch e { Y }`: Y runs with e the caught Error: the Error X gave back, else the runtime error it raised
	/// (caught_error, try_guard.rs)
	fn lower_try_binding(&self, guarded: Node, fallback: Node, binding: &Node) -> Node {
		let (value, fallback_placeholder) = (TRY_VALUE_PLACEHOLDER, TRY_FALLBACK_PLACEHOLDER);
		self.instantiate_template(
			&format!("(try_tmp_finished={RAN_WITHOUT_ERROR}({{try_tmp_value={value}}}); if try_tmp_finished and not is_error(try_tmp_value) {{try_tmp_value}} else {{\
				{CAUGHT_BINDING_PLACEHOLDER} = {CAUGHT_ERROR}(try_tmp_finished, try_tmp_value); {fallback_placeholder}}})"),
			&[(value, &guarded), (fallback_placeholder, &fallback), (CAUGHT_BINDING_PLACEHOLDER, binding)],
		)
	}

	/// `assert C else X` is 1 when C holds and throws X otherwise, also mid-program (`check C`, card check-assert); without
	/// a message the error says the assertion failed
	fn lower_assert(&self, condition: Node, message: Node) -> Node {
		let message = match message.drop_meta() {
			Node::Empty => Node::Text(format!("{}: {}", crate::warp_parser::ASSERTION_FAILED, condition.serialize())),
			_ => message,
		};
		let condition_placeholder = ASSERT_CONDITION_PLACEHOLDER;
		self.instantiate_template(
			&format!("(if {condition_placeholder} {{1}} else {{{}({TRY_FALLBACK_PLACEHOLDER})}})", crate::wasm_emitter::text_builtins::RAISE),
			&[(condition_placeholder, &condition), (TRY_FALLBACK_PLACEHOLDER, &message)],
		)
	}

	/// The template of an expanded word with the receiver (held once in a temporary) and the further arguments in place
	fn expanded(&self, template: &str, arguments: Vec<Node>) -> Node {
		let number = self.temporaries.get();
		self.temporaries.set(number + 1);
		let temporary = format!("{TEMPORARY}_{number}");
		let body = template.replace(TEMPORARY, &temporary);
		let program = parse(&format!("({temporary}={RECEIVER_PLACEHOLDER}; {body})"));
		arguments.iter().enumerate().fold(program, |program, (index, argument)| {
			let placeholder = if index == 0 { RECEIVER_PLACEHOLDER.to_string() } else { format!("{RECEIVER_PLACEHOLDER}_{}", index + 1) };
			substitute(program, &placeholder, argument)
		})
	}
}

/// `(tmp = xs; loop)` → `(tmp = xs; list_sum(tmp, loop))`
fn dispatched_sum(expanded: Node) -> Node {
	let Node::List(mut items, bracket, separator) = expanded.drop_meta().clone() else { return expanded };
	let [assignment, sum_loop] = items.as_mut_slice() else { return Node::List(items, bracket, separator) };
	let Node::Key(temporary, Op::Assign, _) = assignment.drop_meta() else { return Node::List(items, bracket, separator) };
	let list = temporary.as_ref().clone();
	*sum_loop = Node::List(vec![Node::Symbol(LIST_SUM.to_string()), list, sum_loop.clone()], Bracket::Round, Separator::None);
	Node::List(items, bracket, separator)
}

/// `a == b`, `a < b`, `a and b`: where a library word's argument stops
fn is_comparison(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, op, _) if op.is_comparison() || op.is_logical())
}

/// The leftmost operand of a comparison chain, replaced: `"a" == b and c` → `f("a") == b and c`
fn with_first_comparand(node: Node, replace: &dyn Fn(Node) -> Node) -> Node {
	match node {
		Node::Key(left, op, right) if op.is_comparison() || op.is_logical() => Node::Key(Box::new(with_first_comparand(*left, replace)), op, right),
		Node::Meta { node, data } if is_comparison(&node) => Node::Meta { node: Box::new(with_first_comparand(*node, replace)), data },
		operand => replace(operand),
	}
}

pub(crate) fn substitute(node: Node, placeholder: &str, replacement: &Node) -> Node {
	match node {
		Node::Symbol(name) if name == placeholder => replacement.clone(),
		Node::Key(left, op, right) => Node::Key(Box::new(substitute(*left, placeholder, replacement)), op, Box::new(substitute(*right, placeholder, replacement))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| substitute(item, placeholder, replacement)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(substitute(*node, placeholder, replacement)), data },
		other => other,
	}
}

/// A zero written with a decimal point, `0.0`
fn is_float_zero(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Number(crate::extensions::numbers::Number::Float(value)) if *value == 0.0)
}

/// An operand as written in a warning: a call `f 4` / `f(4)` as f(4), anything else as normalize writes it
fn call_text(node: &Node) -> String {
	match node.drop_meta() {
		Node::List(items, Bracket::None | Bracket::Round, _) if items.len() > 1 && matches!(items[0].drop_meta(), Node::Symbol(_)) => {
			format!("{}({})", items[0].drop_meta().name(), items[1..].iter().map(call_text).collect::<Vec<_>>().join(", "))
		}
		_ => crate::normalize::operand_text(node),
	}
}

fn is_marker(node: &Node, marker: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if name == marker)
}

/// The untyped parameters of `def f(a, b) {…}` (any function keyword): the head `f (a, b)` holds them in a group
/// The name of `def f(a, b) {…}`: f
fn keyword_definition_name(items: &[Node]) -> Option<String> {
	keyword_definition_head(items)?.first().map(Node::name)
}

/// The head items of `def f(a, b) {…}` (any function keyword): f and its parameters
fn keyword_definition_head(items: &[Node]) -> Option<&[Node]> {
	let (keyword, head) = match items {
		[keyword, head, _body] => (keyword, head),
		// `def ((f (m)) {…})`: the head and the body as one group
		[keyword, definition] => match definition.drop_meta() {
			Node::List(parts, Bracket::Round, _) if parts.len() == 2 => (keyword, &parts[0]),
			_ => return None,
		},
		_ => return None,
	};
	let Node::Symbol(keyword) = keyword.drop_meta() else { return None };
	if !crate::operators::is_function_keyword(keyword) {
		return None;
	}
	let Node::List(head_items, Bracket::Round, _) = head.drop_meta() else { return None };
	Some(head_items)
}

fn keyword_definition_parameters(items: &[Node]) -> Option<Vec<String>> {
	let head_items = keyword_definition_head(items)?;
	let parameters = head_items.iter().skip(1).flat_map(|item| match item.drop_meta() {
		Node::List(group, Bracket::Round, _) => group.clone(),
		other => vec![other.clone()],
	});
	Some(parameters.filter_map(|parameter| match parameter.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(name, Op::Assign, _) => Some(name.name()),
		_ => None,
	}).collect())
}

/// The word `of` (`first of xs`, `name of person`)
fn is_of(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if word == OF_WORD)
}
