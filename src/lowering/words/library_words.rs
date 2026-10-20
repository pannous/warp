//! The basic text and list words (`upper`, `first`, `sum`, `split` …) in their three spellings: `x.word`, `x.word(args)`,
//! `word(x, args)` and `word x args` all become the call `word(x, args)`.
//! Words that need no runtime function are expanded to source here (`first`, `last`, `sum`); the others are calls that the
//! emitter resolves (lib/prelude.warp's `from warp` lines). A user function or variable of the same name wins.
//!
//! An unknown `.word` after a name, a text or a list is a loud error (`undefined function: word`), never silent data.

use super::nodes::{
	call, children_rewritten, if_then_else, in_block_as_written, is_assigned_data, is_words_call, key, prelude_call,
	with_parts_rewritten, word_headed, Counter,
};
use crate::analyzer::{call_name, counting_method};
use crate::compiler::context::Context;
use crate::errors::diagnostic::Diagnostic;
use crate::list_mutations::is_list_mutating_method;
use crate::node::{parens, symbol, text, Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::Kind;
use crate::warp_parser::{parse_template, ASSERT_MARKER, TRY_MARKER};
use crate::wasm_emitter::{CAUGHT_ERROR, RAN_WITHOUT_ERROR};
use crate::words::COPY_WORD;
use crate::words::COUNT_WORD;
use crate::words::FLOAT_WORD;
use crate::words::FOR_WORD;
use crate::words::IN_WORD;
use crate::words::LIST_WORD;
use crate::words::LOG_WORD;
use crate::words::NUMBER_WORD;
use crate::words::{MAP_KEYS, RAISE_WORD, SUM_WORD};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

mod objects;
pub(crate) use objects::*;
mod counts;
pub use counts::*;
mod function_methods;
pub use function_methods::*;
mod guards;
mod similarity;
pub use similarity::*;
mod templates;
pub(crate) use templates::*;

/// The map words, named after their runtime functions: `m.keys()`, `m.values()`, `x in xs` (`xs.has(x)`, Python's and
/// JavaScript's spellings too) and `m.get(k)`/`m.get(k, default)`. A key is found by its letters, quoted or not.
pub const MAP_VALUES: &str = "map_values";
/// map_entries(xs): the `key:value` entries of a map as a list (a one-entry map `{a:1}` is the entry itself), else xs
pub const MAP_ENTRIES: &str = "map_entries";
pub const COLLECTION_CONTAINS: &str = "collection_contains";
/// collection_position(xs, x): `xs.index_of(x)`, the 1-based position of x in a list (0 when absent), 1/0 for a map key
pub const COLLECTION_POSITION: &str = "collection_position";
pub const MAP_GET_OR: &str = "map_get_or";
/// map_without(map, key): the map without the key's entry, what `m.remove(k)` stores in m (analyzer removed_key)
pub const MAP_WITHOUT: &str = "map_without";
pub const MAP_WORD_FUNCTIONS: [&str; 8] = [
	MAP_KEYS,
	MAP_VALUES,
	MAP_ENTRIES,
	COLLECTION_CONTAINS,
	COLLECTION_POSITION,
	MAP_GET_OR,
	MAP_WITHOUT,
	crate::one_entry_maps::MAP_OF_ENTRY,
];
/// The modules named after a type, whose words `x.word` calls on a value of it (lib/list.warp, text.warp)
/// The result type of a builtin that gives no value (`print(values...): nothing from emitter`)
const NOTHING_RESULT: &str = "nothing";
/// The spellings of a result read off an argument (decision same-as-result)
const ARGUMENT_RESULTS: [(&str, ArgumentResult); 3] = [
	("same as ", ArgumentResult::Same),
	("element of ", ArgumentResult::Element),
	("value of ", ArgumentResult::Element),
];
const TYPE_MODULES: [&str; 2] = ["list", "text"];
/// The name `catch e` binds, in lower_try_binding's template
const CAUGHT_BINDING_PLACEHOLDER: &str = "caught_binding";
/// lib/prelude.warp's words of `count x in y`: how often an item or character occurs, how often a part of a text
const COUNT_ITEM: &str = "count_item";
const COUNT_PART: &str = "count_of";
/// lib/prelude.warp's byte list of a text, `t.bytes`
pub(crate) const BYTE_LIST: &str = "bytes";

/// The library's words besides the runtime, expanded and prelude words: the text builtins warning and trim, the type
/// word list and copy; their other spellings are lib/prelude.warp's alias lines
const OTHER_LIBRARY_WORDS: [&str; 4] =
	[crate::wasm_emitter::text::text_builtins::WARNING, LIST_WORD, "trim", COPY_WORD];
/// `list_sum(list, loop)`: the sum of a list variable as one operation the emitter dispatches (wasm_emitter/lists/list_dispatch.rs):
/// a typed list sums its array, any other list runs the loop `sum` always lowered to
pub const LIST_SUM: &str = "list_sum";
/// `t.chars`: the list of a text's characters, a grapheme cluster each (`char` is short for character)
pub const CHARS_WORD: &str = "chars";
/// The list of a text's code points, which `for codepoints in t` and `for each codepoint in t` walk
pub const CODE_POINT_LIST: &str = "code_point_list";
/// The runtime functions behind them: text_graphemes(text), the list of its characters; text_chars(text), of its code points
pub const TEXT_GRAPHEMES: &str = "text_graphemes";
pub const TEXT_CODE_POINTS: &str = "text_chars";

/// A word the emitter implements as a runtime function: its `from warp` line in lib/prelude.warp
fn runtime_word(name: &str) -> Option<&'static crate::ffi::ForeignLine<'static>> {
	crate::ffi::runtime_word_lines().iter().find(|line| line.name == name)
}

/// The argument at `index` a call leaves out: its parameter's default (the space `s.split` splits at), else ø
fn left_out_argument(word: &str, index: usize) -> Node {
	let default = runtime_word(word).and_then(|line| line.defaults().nth(index).flatten());
	default.map_or(Node::Empty, |default| crate::warp_parser::parse(default).drop_meta().clone())
}

/// The bases with an exact libm logarithm: `log(1000, 10)` is log10(1000) = 3, where ln(1000)/ln(10) is 2.9999999999999996
const EXACT_LOG_BASES: [(i64, &str); 2] = [(10, "log10"), (2, "log2")];

/// `log(value, base)`: libm's log takes one argument, so ln(value)/ln(base), exact for the EXACT_LOG_BASES;
/// a list literal is the log of each item
fn log_to_base(log_word: &Node, value: &Node, base: &Node) -> Node {
	if let Node::List(values, Bracket::Square, separator) = value.drop_meta() {
		let logs = values.iter().map(|item| log_to_base(log_word, item, base)).collect();
		return Node::List(logs, Bracket::Square, separator.clone());
	}
	let exact_log = EXACT_LOG_BASES.iter().find(|(exact_base, _)| {
		matches!(base.drop_meta(), Node::Number(crate::extensions::numbers::Number::Int(number)) if number == exact_base)
	});
	if let Some((_, exact_word)) = exact_log {
		return parens(vec![symbol(exact_word), value.clone()]);
	}
	let log_of = |argument: &Node| parens(vec![log_word.clone(), argument.clone()]);
	key(log_of(value), Op::Div, log_of(base))
}

/// `field_with(object, "name", value)`: a copy of the object with the field set; what `object.name = value` lowers to
pub const FIELD_WITH: &str = "field_with";
/// instance_copy(x): a new instance with x's field values (`p.copy()`, Kotlin's `p.copy(y = 5)`); any other value is x
pub const INSTANCE_COPY: &str = "instance_copy";
/// The named argument of `copy` asking for a shallow copy (P205)
pub const SHALLOW: &str = "shallow";

/// `instance_copy(object, shallow)`
pub fn copy_call(object: Node, shallow: Node) -> Node {
	call(INSTANCE_COPY, vec![object, shallow])
}

/// The value of `shallow: v` / `shallow = v`, or a positional `v`
pub fn shallow_flag(argument: &Node) -> Node {
	match argument.drop_meta() {
		Node::Key(name, Op::Colon | Op::Assign, value) if name.drop_meta().name() == SHALLOW => value.as_ref().clone(),
		other => other.clone(),
	}
}

/// Source of the words expanded here, with their number of arguments; `word_argument` is the receiver, `hidden_value` a
/// temporary that holds it once (each `hidden_x` becomes the temporary `word·x·3`), `word_argument_2` … the arguments after the receiver. The words written in warp
/// (first, last, as_list, round_to, replace, is_digit …) are lib/prelude.warp's. These two stay: sum's list_sum is the
/// hook of the typed-list fast path (typed_list_finding.rs), and `x!` as a prelude call was 12% slower in a loop
/// (card sum-list: the engine does not inline calls)
const EXPANDED_WORDS: [(&str, usize, &str); 2] = [
	(
		SUM_WORD,
		1,
		"(hidden_sum=0; for hidden_item in hidden_value {hidden_sum = hidden_sum + hidden_item}; hidden_sum)",
	),
	// `x!` (mutation.rs): the value, a loud error when it is ø; an Error value stays that Error
	(crate::words::UNWRAP, 1, "if hidden_value == ø then error \"unwrapped ø\" else hidden_value"),
];

/// An expanded word as the warp definition it stands for, for readers that take definitions (the law type model):
/// `sum(hidden_value) := (hidden_sum=0; …; hidden_sum)`
pub(crate) fn expanded_word_definition(word: &str) -> Option<String> {
	let (name, arity, template) = EXPANDED_WORDS.iter().find(|(name, _, _)| *name == word)?;
	let after_receiver = (2..=*arity).map(|index| format!(", {RECEIVER_PLACEHOLDER}_{index}"));
	let parameters: String = std::iter::once(TEMPORARY.to_string()).chain(after_receiver).collect();
	Some(format!("{name}({parameters}) := {template}"))
}
const DICT_WORD: &str = "dict";
/// The type of a parameter taking a function value: `c: function`
const FUNCTION_TYPE_WORD: &str = "function";
/// lib/prelude.warp's fromkeys(ks, value), also called on the class as Python's `dict.fromkeys(ks, value)`
const FROMKEYS: &str = "fromkeys";
/// Conversions a method may name, `x.string` being `string(x)`
pub(crate) const CONVERSION_METHODS: [&str; 4] = ["string", "str", "int", "float"];
const LIST_CONSTRUCTORS: [&str; 4] = ["listOf", "mutableListOf", "arrayOf", "arrayListOf"];
/// Words that mean another word with that many arguments (receiver included): `round(x, 3)`, `x.round(3)` is x rounded
/// to 3 digits after the point (`round(x)` stays the builtin); `xs.first(3)`, `first 3 of xs` the first 3 items,
/// `last(xs, 2)` the last 2; Python's `list(x)` as_list(x), its own word: the prelude's `list` would rename the type
/// `x as list` too (lib/prelude.warp)
const ARITY_VARIANTS: [(&str, usize, &str); 5] = [
	("round", 2, "round_to"),
	("first", 2, "first_items"),
	("last", 2, "last_items"),
	(LIST_WORD, 1, AS_LIST),
	("tuple", 1, AS_LIST),
];
const AS_LIST: &str = "as_list";
/// Methods a later pass lowers to prelude words: `x.pop(k)` (and `delete x[k]`, mutation::lower_deletes) to popped_item
/// and popped_rest, `x.clear()` to cleared, `x.insert(v, at: i)` to inserted (list_mutations removed_key,
/// cleared_collection, lowered_insert)
const POPPED_WORDS: &[&str] = &["popped_item", "popped_rest"];
const LOWERED_TO_PRELUDE: [(&str, &[&str]); 4] =
	[("pop", POPPED_WORDS), ("delete", POPPED_WORDS), ("clear", &["cleared"]), ("insert", &["inserted"])];
const MODULE_SOURCE_WORD: &str = "module_source";
const LEFT_OPERAND_PLACEHOLDER: &str = "operand_placeholder_left";
const RIGHT_OPERAND_PLACEHOLDER: &str = "operand_placeholder_right";
const OPERAND_TEMPORARY: &str = "operand";
/// The variable holding a list while one of its elements changes: `place·0`
const PLACE_TEMPORARY: &str = "place·";
const RECEIVER_PLACEHOLDER: &str = "word_argument";
const LOOKUP_PLACEHOLDER: &str = "word_lookup";
const TEMPORARY: &str = "hidden_value";
const HIDDEN_PREFIX: &str = "hidden_";
const WORD_BASE: &str = "word";
const SAFE_PREFIX: &str = "safe_tmp_";
const SAFE_BASE: &str = "safe";

/// A library word's result when it is always a text or a list: its signature's (upper, split, map_keys), a text
/// builtin's (trim), list_extend's (`xs + ys` of lists)
pub fn result_kind(word: &str) -> Option<Kind> {
	call_result_kind(word, || None)
}

/// A library word's result for a first argument of the kind `argument` gives: the result of its overload for that
/// kind (`reverse(t: text): text`, card typed-builtins), else the one all its overloads share. When the overloads
/// differ, a Node of run-time kind gives a Node and any other value no overload is typed for (a name, a range) the
/// first line's result, a list's. The argument is inferred only then
pub fn call_result_kind(word: &str, argument: impl FnOnce() -> Option<Kind>) -> Option<Kind> {
	let word = canonical_spelling(word);
	let overloads: Vec<_> = crate::ffi::runtime_word_lines().iter().filter(|line| line.name == word).collect();
	let Some(first) = overloads.first() else {
		return match word {
			crate::wasm_emitter::lists::list_ops::LIST_EXTEND => Some(Kind::List),
			_ => {
				crate::wasm_emitter::text::text_builtins::text_builtin_kind(word, 1).filter(|kind| *kind == Kind::Text)
			}
		};
	};
	if overloads.iter().all(|line| line_result_kind(line) == line_result_kind(first)) {
		return line_result_kind(first);
	}
	let fitting = match argument()? {
		Kind::Empty => return Some(Kind::Empty),
		Kind::Codepoint => Kind::Text,
		kind => kind,
	};
	let typed_for = |line: &&&crate::ffi::ForeignLine| line.first_type().and_then(crate::analyzer::type_word_kind);
	line_result_kind(overloads.iter().find(|line| typed_for(line) == Some(fitting)).unwrap_or(first))
}

/// How a builtin's result follows one of its arguments (decision same-as-result): its very type, `same as object`, or
/// the type of its elements, `element of xs`, `value of m`
#[derive(Clone, Copy)]
pub enum ArgumentResult {
	Same,
	Element,
}

/// The argument a builtin line's result follows, and how: `field_with(object, name, value): same as object` → (Same,
/// 0), `map_get_or(m, key, default): value of m` → (Element, 0)
pub fn argument_result(word: &str) -> Option<(ArgumentResult, usize)> {
	let word = unaliased(word);
	let line = crate::ffi::typed_word_lines().find(|line| line.name == word)?;
	let (how, parameter) = ARGUMENT_RESULTS
		.iter()
		.find_map(|(spelling, how)| line.result.strip_prefix(spelling).map(|parameter| (*how, parameter)))?;
	Some((how, line.names().position(|name| name == parameter)?))
}

/// The result of a call of `arguments` values to a builtin the emitter expands, as its line declares it
/// (`count(xs): int from emitter`; print's `nothing` a Node of no value)
pub fn emitted_call_kind(word: &str, arguments: usize) -> Option<Kind> {
	let word = unaliased(word);
	let line = crate::ffi::emitter_word_lines().iter().find(|line| line.name == word && line.takes(arguments))?;
	match line.result {
		NOTHING_RESULT => Some(Kind::Empty),
		// a type's name (`type(3)` is int), held as no other value is
		crate::words::SYMBOL_WORD => Some(Kind::Symbol),
		// `number("2.5")` an int or a float, `any(xs, f)` a bool the emitter marks: decided where it is made
		crate::words::NUMBER_WORD | crate::analyzer::BOOL_TYPE => None,
		// a Node of any kind, decided at run time (`removed_value(x, k)`)
		crate::words::ANY_TYPE => Some(Kind::Empty),
		_ => line_result_kind(line).or_else(|| crate::analyzer::type_word_kind(line.result)),
	}
}

fn line_result_kind(line: &crate::ffi::ForeignLine) -> Option<Kind> {
	match line.result {
		"text" => Some(Kind::Text),
		result if result.starts_with(LIST_WORD) => Some(Kind::List),
		_ => None,
	}
}

/// `field_with(m, key, value)` updating the variable m itself: what `m.key = value` lowers to (field_assignment)
pub fn is_field_update_of(name: &str, value: &Node) -> bool {
	let Node::List(items, _, _) = value.drop_meta() else {
		return false;
	};
	let [word, map, _, _] = items.as_slice() else {
		return false;
	};
	word.name() == FIELD_WITH && map.is_symbol(name)
}

pub fn is_runtime_word(name: &str) -> bool {
	runtime_word(name).is_some()
}

/// A word of the library, under any of its spellings: `codepoint(c)` calls it, it is no type constructor
pub fn is_library_word(name: &str) -> bool {
	is_runtime_word(name) || canonical_word(name).is_some()
}

/// A function every program has: a library word, an extremum, a rounding function or a text builtin. A class's method
/// of that name adds to it, other values keep calling it (notes/dispatch.md "Methods")
pub fn is_standard_function(name: &str) -> bool {
	is_library_word(name)
		|| crate::min_max::is_extremum(name)
		|| crate::wasm_emitter::ROUNDING_FUNCTIONS.contains(&name)
		|| crate::wasm_emitter::text::text_builtins::is_text_builtin(name)
}

/// A standard word that calls the values after it: a standard function, a counting word or an output word
/// (`count sorted xs`, `max sorted xs`, `print sorted xs`)
pub fn is_calling_word(name: &str) -> bool {
	is_standard_function(name)
		|| crate::analyzer::is_counting_word(name)
		|| crate::wasm_emitter::OUTPUT_WORDS.contains(&name)
}

/// A library word of one argument takes the name after it as that argument inside an operand, as a defined function
/// does: `word == reverse word` compares with `reverse(word)`, `1 == first xs` with `first(xs)`. A type word stays a
/// type: `x: list int`
pub fn is_operand_word(name: &str) -> bool {
	canonical_word(name).is_some_and(|word| arity(word) == 1 && !crate::type_tests::is_builtin_type_word(word))
}

/// A library word that needs one value, its further parameters optional: `reverse xs`, `split text` (its separator
/// optional), no type word
pub fn needs_one_value(name: &str) -> bool {
	canonical_word(name)
		.is_some_and(|word| fewest_arguments(word) == 1 && !crate::type_tests::is_builtin_type_word(word))
}

/// A library word that is a function of one value, standing as a function value (`map floor xs`, `map reverse xs`):
/// one needing one value, one of one typed value (`floor`, `sin`) or a counting word (`map count xs`)
pub fn is_one_value_function(name: &str) -> bool {
	needs_one_value(name)
		|| crate::lowering::library_signatures::typed_first_arity(name) == Some(1)
		|| crate::analyzer::is_counting_word(name)
}

fn fewest_arguments(word: &str) -> usize {
	let optional = runtime_word(word).map_or(0, |line| line.defaults().flatten().count());
	arity(word) - optional
}

/// The spelling word lists name a library word by (`sorted` → sort); any other name as it is
pub fn canonical_spelling(name: &str) -> &str {
	canonical_word(name).unwrap_or(name)
}

fn canonical_word(name: &str) -> Option<&'static str> {
	let name = unaliased(name);
	let runtime = crate::ffi::runtime_word_lines().iter().map(|line| line.name);
	let expanded = EXPANDED_WORDS.into_iter().map(|(word, _, _)| word);
	let built_in = runtime.chain(expanded).chain(OTHER_LIBRARY_WORDS).find(|word| *word == name);
	built_in.or_else(|| crate::modules::prelude_word(name))
}

/// The word a prelude alias names (`size` → count), else the name itself
pub(crate) fn unaliased(name: &str) -> &str {
	let aliases = crate::modules::prelude_aliases();
	aliases.iter().find(|(alias, _)| alias == name).map_or(name, |(_, word)| word.as_str())
}

pub(crate) fn arity(word: &str) -> usize {
	let expanded = EXPANDED_WORDS.iter().map(|(name, arity, _)| (*name, *arity));
	let runtime = crate::ffi::runtime_word_lines().iter().map(|line| (line.name, line.arity()));
	let built_in = runtime.chain(expanded).find(|(name, _)| *name == word).map(|(_, arity)| arity);
	built_in.or_else(|| crate::modules::prelude_word_arity(word)).unwrap_or(1)
}

/// The library words a name may stand for once this pass has run: its canonical spelling (`isdigit` → is_digit) and,
/// its ARITY_VARIANTS (`round(x, 2)` round_to). modules::resolve, which runs before, loads the prelude words by them
pub fn words_spelled_by(name: &str) -> Vec<&'static str> {
	let variants = ARITY_VARIANTS.iter().filter(|(word, _, _)| *word == name).map(|(_, _, variant)| *variant);
	let lowered =
		LOWERED_TO_PRELUDE.iter().filter(|(word, _)| *word == name).flat_map(|(_, words)| words.iter().copied());
	canonical_word(name).into_iter().chain(variants).chain(lowered).collect()
}

pub fn lower(node: Node) -> Node {
	let node = bind_read_data_objects(node);
	let context = crate::analyzer::function_definitions(&node);
	let mut shadowed: HashSet<String> = context.user_functions.keys().cloned().collect();
	// `sum·1` (an arity overload), `sum·point` of a declared trait's operation: the program defines `sum`, no library
	// word; a class's method `sum·Bag` of the generic function sum keeps the library's sum for other values
	let traits = crate::traits::Traits::of(&node);
	let replaces_word =
		|operation: &str, variant: &str| variant.parse::<usize>().is_ok() || traits.is_operation(operation);
	let variants: Vec<String> = shadowed
		.iter()
		.filter_map(|name| name.split_once(crate::traits::WITNESS_SEPARATOR))
		.filter(|(operation, variant)| replaces_word(operation, variant))
		.map(|(operation, _)| operation.to_string())
		.collect();
	shadowed.extend(variants);
	let functions = shadowed.clone();
	crate::type_tests::collect_outer_assigned_names(&node, &mut shadowed);
	let mut assigned = AssignedObjects::default();
	collect_assigned_objects(&node, &mut assigned);

	let plain = assigned.plain.difference(&assigned.rebound).cloned().collect();
	let objects = assigned.objects.into_iter().filter_map(|(name, literal)| Some((name, literal?))).collect();
	let instances = crate::traits::InstanceTypes::of(&node);
	let call_results = call_results(&node, &context);
	let times = crate::time::time_variables(&node);
	let entry_names = entry_names(&node);
	Lowering {
		context,
		shadowed: RefCell::new(shadowed),
		functions,
		program: crate::program_names::ProgramNames::of(&node),
		objects,
		plain,
		instances,
		parameters: RefCell::new(vec![]),
		function_parameters: RefCell::new(vec![]),
		defining: RefCell::new(vec![]),
		call_results,
		times,
		entry_names,
		error_classes: crate::lowering::error_classes::ErrorClasses::of(&node),
		temporaries: Counter::default(),
	}
	.expand(node)
}

/// The variables assigned what a user function returns (`result = parse_json(text)`): any may hold an object
fn call_results(node: &Node, context: &Context) -> HashSet<String> {
	let mut names = HashSet::new();
	node.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = part {
			let calls_user_function = matches!(value.drop_meta(), Node::List(items, Bracket::Round, _)
				if items.first().and_then(Node::symbol_name).is_some_and(|function| context.user_functions.contains_key(function)));
			if let (Node::Symbol(name), true) = (target.drop_meta(), calls_user_function) {
				names.insert(name.clone());
			}
		}
	});
	names
}

/// `raise X`, `throw X`, `fail X`, `panic X`, `raise(X)`: the call `raise(X)`; `raise error "m"` raises the message m.
/// The word `name` means with that many arguments
fn arity_variant(name: &str, argument_count: usize) -> Option<&'static str> {
	let variant = ARITY_VARIANTS.iter().find(|(word, count, _)| *word == name && *count == argument_count);
	variant.map(|(_, _, variant)| *variant)
}

/// A word the program defines (`shadowed`) is its own
fn raise_call(
	items: &[Node],
	shadowed: &HashSet<String>,
	error_classes: &crate::lowering::error_classes::ErrorClasses,
) -> Option<Node> {
	let [word, raised] = items else { return None };
	let is_raise_word = |raise: &&str| word.is_symbol(raise) && !shadowed.contains(*raise);
	if !crate::errors::run_time::RAISE_WORDS.iter().any(is_raise_word) {
		return None;
	}
	if let Some(raised_as) = error_classes.raised_instance(raised, |name| shadowed.contains(name)) {
		return Some(raised_as);
	}
	let message = crate::errors::run_time::returned_error_message(raised).unwrap_or(raised).clone();
	// the message is a text, so `catch e` reads it whatever was raised (`raise 42`: "42"); a word (`raise oops`) is its name
	let message = match message.drop_meta() {
		Node::Text(_) | Node::Symbol(_) => message,
		_ => call(crate::wasm_emitter::text::text_builtins::TEXT_FORM, vec![message]),
	};
	Some(call(RAISE_WORD, vec![message]))
}

/// The field and step of `x++` / `x--` after a dot
fn field_step(method: &Node) -> Option<(Node, Op)> {
	match method.drop_meta() {
		Node::Key(field, op @ (Op::Inc | Op::Dec), _) if field.symbol_name().is_some() => {
			Some((field.as_ref().clone(), *op))
		}
		_ => None,
	}
}

/// A place deeper than a variable, its element or its field (`b.n += 1` sets a struct field in place): `p.b.n`,
/// `bags#1.n`, `xs#1#2`
fn is_nested_place(place: &Node) -> bool {
	match place.drop_meta() {
		Node::Key(base, Op::Dot | Op::Hash, _) => base.symbol_name().is_none(),
		_ => false,
	}
}

/// `place += 1` for `place++`, `place -= 1` for `place--`
pub(crate) fn stepped(place: Node, step: Op) -> Node {
	let update = if step == Op::Inc { Op::AddAssign } else { Op::SubAssign };
	key(place, update, Node::int(1))
}

/// The word of a method and its arguments: `x.word`, `x.word(args)`
fn method_parts(method: &Node) -> Option<(&Node, &str, Vec<Node>)> {
	let (word_node, arguments) = match method.drop_meta() {
		Node::Symbol(_) => (method, vec![]),
		Node::List(items, _, _) if word_headed(items).is_some() => (&items[0], items[1..].to_vec()),
		_ => return None,
	};
	let name = word_node.symbol_name()?;
	Some((word_node, name, arguments))
}

/// `x.sqrt`, `x.abs()`: the prefix operator the word names (`sqrt x`, `abs x`) applied to the receiver
fn prefix_operator_method(receiver: &Node, name: &str) -> Option<Node> {
	let (_, operator) = crate::warp_parser::PREFIX_OPERATOR_WORDS.into_iter().find(|(word, _)| *word == name)?;
	Some(key(Node::Empty, operator, receiver.clone()))
}

/// The module named after a type (lib/list.warp, text.warp) that defines the word
pub fn type_module_defining(word: &str) -> Option<&'static str> {
	let defines = |module: &&str| {
		crate::modules::std_module_definitions(module).is_some_and(|words| words.iter().any(|known| known == word))
	};
	TYPE_MODULES.into_iter().find(defines)
}

/// A standard module's word the program loaded and does not define itself
fn is_loaded_module_word(name: &str) -> bool {
	crate::compiler::compile_context::read(|context| context.module_definitions.contains(name))
}

/// Whether the receiver of `x.word` has a member `word` of its own (level 1 of the one lookup, notes/dispatch.md)
enum OwnMember {
	/// known at compile time: the literal's entry, the field of its declared class
	Has,
	/// an object known to lack it: the word, else the field read that fails
	Lacks,
	/// a value known only at run time, which may be an object with it
	Maybe,
	/// no object: the word, else an error
	Not,
}

/// What `x.word(args)` means for any value, below a member of the receiver's own (levels 2 to 4)
enum WordMeaning {
	/// a call; `typed`: a word whose result type the code after it relies on, which a run-time choice would hide
	Call {
		call: Node,
		typed: bool,
	},
	/// a field read, which answers it at run time (a swizzle `p.zy`)
	FieldRead,
	/// a word a later step answers on the dot itself (`xs.count`, `xs.pop()`, `f.effects`)
	Kept,
	Undefined,
}

/// A variable or an element of one (`xs#i`, as a loop reads its element): read twice, it is the same value
fn is_pure_read(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(_) => true,
		Node::Key(list, Op::Hash, index) => {
			list.symbol_name().is_some() && matches!(index.drop_meta(), Node::Symbol(_) | Node::Number(_))
		}
		_ => false,
	}
}

/// The call `word(receiver, arguments…)`
fn word_call(word_node: &Node, receiver: &Node, arguments: Vec<Node>) -> Node {
	Node::List([vec![word_node.clone(), receiver.clone()], arguments].concat(), Bracket::Round, Separator::None)
}

struct Lowering {
	context: Context,
	/// The names the program defines where the lowering is: its functions, the variables outside functions and those
	/// the function being lowered assigns (a local in one function hides no library word in another)
	shadowed: RefCell<HashSet<String>>,
	/// The functions the program defines, the only names that hide a library word after a dot: a variable named like
	/// the word is no member of the receiver (`t.bytes` with `bytes = …` still reads the bytes)
	functions: HashSet<String>,
	/// The program's functions at the numbers of arguments they take
	program: crate::program_names::ProgramNames,
	objects: HashMap<String, Node>,
	/// Variables assigned plain values (AssignedObjects::plain)
	plain: HashSet<String>,
	/// Values of declared types (`p:person`, `first = (sort people)#1`) read their fields like object literals
	instances: crate::traits::InstanceTypes,
	/// The untyped parameters of the definitions being lowered: any of them may hold an object, so `p.width` reads a field
	parameters: RefCell<Vec<String>>,
	/// The parameters declared `function` of the definitions being lowered: `c.name` reads the function's name, never a
	/// class's method name()
	function_parameters: RefCell<Vec<String>>,
	/// The functions being defined, innermost last, each with its parameters: `start(l) := l.start` reads the field
	defining: RefCell<Vec<(String, Vec<String>)>>,
	/// Variables assigned the result of a user function call (call_results)
	call_results: HashSet<String>,
	/// Variables holding a time (time::time_variables): `(x + 1 hour).hour` reads the field
	times: HashSet<String>,
	/// The keys the program's map literals give entries: only a counting word named so may read an entry (`{size: 9}`)
	entry_names: HashSet<String>,
	/// The program's error classes, which `except C` and `raise C(…)` name (decision error-classes)
	error_classes: crate::lowering::error_classes::ErrorClasses,
	temporaries: Counter,
}

impl Lowering {
	fn expand(&self, node: Node) -> Node {
		match node {
			// `data (a.b)` stays the code as written, the Node the run-time `parse("a.b")` gives (card data-quote-lowered)
			quoted if crate::blocks::quoted_data(&quoted).is_some() => quoted,
			Node::List(items, Bracket::Round, _) if items.len() == 3 && items[0].is_symbol(TRY_MARKER) => {
				self.lower_try(self.expand(items[1].clone()), self.expand(items[2].clone()))
			}
			// `catch e { … }`: the fallback reads the caught Error as e (P67)
			Node::List(items, Bracket::Round, _) if items.len() == 4 && items[0].is_symbol(TRY_MARKER) => {
				self.lower_try_binding(self.expand(items[1].clone()), self.expand(items[2].clone()), &items[3])
			}
			Node::List(items, Bracket::Round, _) if items.len() == 3 && items[0].is_symbol(ASSERT_MARKER) => {
				self.lower_assert(self.expand(items[1].clone()), self.expand(items[2].clone()))
			}
			// `def f(m) { m.a }`: the body sees the parameters, as `f(m) := m.a` does
			Node::List(items, bracket, separator) if let Some((name, parameters)) = keyword_definition(&items) => {
				let items = self
					.within_function(name, parameters, || items.into_iter().map(|item| self.expand(item)).collect());
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
			Node::Key(left, op @ (Op::Inc | Op::Dec), _)
				if matches!(left.drop_meta(), Node::Key(_, Op::Dot | Op::Hash, _)) =>
			{
				self.expand(stepped(*left, op))
			}
			// `bags#1.n += 4`, `p.b.n += 2`, `xs#1#2 += 1`: `place = place + 4`, stored back into the place
			Node::Key(left, op, right) if op.is_compound_assign() && is_nested_place(&left) => {
				let combined = Node::Key(left.clone(), op.base_op(), right);
				let left = self.expand(*left);
				let combined = self.expand(combined);
				self.stored(&left, combined.clone()).unwrap_or(key(left, Op::Assign, combined))
			}
			Node::Key(left, Op::Dot, right) => {
				let (left, right) = (self.expand(*left), self.expand_method(*right));
				self.method_call(&left, &right).unwrap_or(key(left, Op::Dot, right))
			}
			Node::Key(left, Op::Assign, right) => {
				if let Some(chained) = self.chained_field_assignment(&left, &right) {
					return chained;
				}
				let written = *left;
				let left = self.expanded_place(written.clone());
				if let Some(set) = self.functional_setter(&written, &left, &right) {
					return set;
				}
				let right = self.in_definition(&left, *right);
				self.field_assignment(&left, &right).unwrap_or(key(left, Op::Assign, right))
			}
			Node::Key(left, Op::SafeDot, right) => {
				let (receiver, word) = (self.expand(*left), *right);
				self.safe_lookup(receiver, word)
			}
			Node::Key(left, op @ (Op::Similar | Op::Rough), right) => {
				self.lower_similar(op, self.expand(*left), self.expand(*right))
			}
			Node::Key(left, Op::Coalesce, right) => self.lower_coalesce(self.expand(*left), self.expand(*right)),
			Node::Key(left, Op::Define, right) => {
				let left = self.expanded_head(*left);
				let right = self.in_definition(&left, *right);
				key(left, Op::Define, right)
			}
			// P66: `x / 0.0`, a float division by a written zero, is IEEE's ∞ (an exact division by zero is divide_by_zero)
			Node::Key(left, Op::Div, right) if is_float_zero(&right) => {
				let as_float = |operand: Node| key(operand, Op::As, symbol(FLOAT_WORD));
				key(as_float(self.expand(*left)), Op::Div, as_float(*right))
			}
			other => children_rewritten(other, |child| self.expand(child)),
		}
	}

	/// The left side of `head := right`: the program's own function keeps its name, whatever its parameters look like
	/// as a call (`get [int] xs` of `fn get(int[] xs)` is no call of the library's get)
	fn expanded_head(&self, head: Node) -> Node {
		match head {
			Node::List(items, bracket @ Bracket::Round, separator)
				if items
					.first()
					.and_then(Node::symbol_name)
					.is_some_and(|name| self.program.function_takes(name, 0).is_some()) =>
			{
				let mut items = items.into_iter();
				let name = items.next().into_iter();
				Node::List(name.chain(items.map(|item| self.expand(item))).collect(), bracket, separator)
			}
			other => self.expand(other),
		}
	}

	/// The target of `place = value`: `m.title = v` of an object writes its field, with no run-time choice between its
	/// member and the word title, unless the program defines the setter
	fn expanded_place(&self, place: Node) -> Node {
		match place {
			Node::Key(receiver, Op::Dot, field) if matches!(field.drop_meta(), Node::Symbol(_)) => {
				let receiver = self.expand(*receiver);
				let name = field.name();
				// a setter is the program's function of the receiver and the value (`size(box, n)`)
				let has_setter = self.program.function_takes(&name, 2) == Some(true) && !is_loaded_module_word(&name);
				let may_hold_field =
					matches!(self.own_member(&receiver, &name, &[]), OwnMember::Maybe | OwnMember::Lacks);
				if may_hold_field && !has_setter {
					return field_lookup(&receiver, &name, &field);
				}
				self.method_call(&receiver, &field).unwrap_or(key(receiver, Op::Dot, *field))
			}
			other => self.expanded_head(other),
		}
	}

	/// The right side of `head = right` or `head := right`, with the untyped parameters of a definition head in scope
	fn in_definition(&self, head: &Node, right: Node) -> Node {
		let parameters = crate::traits::untyped_parameters(head);
		let name = match head.drop_meta() {
			Node::List(items, Bracket::Round, _) => items.first().map(Node::name),
			_ => None,
		};
		let mut locals = self.shadowed.borrow().clone();
		collect_assigned_names(&right, &mut locals);
		let outer = self.shadowed.replace(locals);
		let function_scope = self.function_parameters.borrow().len();
		self.function_parameters.borrow_mut().extend(crate::traits::parameters_of_type(head, FUNCTION_TYPE_WORD));
		let lowered = self.within_function(name, parameters, || {
			let body = self.body_statement(head, right);
			// `x = {sum xs}` assigns that data: only its parts are lowered, the block stays no call
			if is_assigned_data(head, &body) {
				return with_parts_rewritten(body, &mut |part| self.expand(part));
			}
			self.expand(body)
		});
		self.function_parameters.borrow_mut().truncate(function_scope);
		self.shadowed.replace(outer);
		lowered
	}

	fn shadows(&self, name: &str) -> bool {
		self.shadowed.borrow().contains(name)
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
		let Node::Symbol(receiver) = receiver.drop_meta() else {
			return false;
		};
		self.defining
			.borrow()
			.last()
			.is_some_and(|(function, parameters)| function == name && parameters.contains(receiver))
	}

	/// `def total(xs){sum xs}`: the body block of a function that is one braceless call is that statement, not the data
	/// `{sum xs}`; an undefined word in it (`{square x}`) is then reported as one
	fn body_statement(&self, head: &Node, right: Node) -> Node {
		let defines_function =
			matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if word_headed(items).is_some());
		match right {
			Node::List(items, Bracket::Curly, Separator::Space)
				if defines_function && items.len() > 1 && word_headed(&items).is_some() =>
			{
				Node::List(
					vec![Node::List(items, Bracket::None, Separator::Space)],
					Bracket::Curly,
					Separator::Semicolon,
				)
			}
			other => other,
		}
	}

	/// The time variables not hidden by a parameter of the definition being lowered: `f(x) := (x - 5).abs` of `x = now`
	fn times_here(&self) -> HashSet<String> {
		let parameters = self.parameters.borrow();
		self.times.iter().filter(|name| !parameters.contains(name)).cloned().collect()
	}

	fn is_parameter(&self, node: &Node) -> bool {
		matches!(node.drop_meta(), Node::Symbol(name) if self.parameters.borrow().contains(name))
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
		let temporary = Node::Symbol(temporary_name(&[OPERAND_TEMPORARY, &self.next_temporary().to_string()]));
		bindings.push(key(temporary.clone(), Op::Assign, operand));
		temporary
	}

	/// The word after a dot, `word` or `word(arguments)`: only the arguments are expanded, the word is not a call of its own
	fn expand_method(&self, method: Node) -> Node {
		match method {
			meta @ Node::Meta { .. } => meta.under_meta(|node| self.expand_method(node)),
			Node::List(items, bracket, separator) if word_headed(&items).is_some() => {
				let mut items = items.into_iter();
				let word = items.next().expect("guarded");
				Node::List(
					[vec![word], items.map(|argument| self.expand(argument)).collect()].concat(),
					bracket,
					separator,
				)
			}
			other => self.expand(other),
		}
	}

	fn library_word(&self, name: &str) -> Option<&'static str> {
		self.word_unless(name, &|word| self.shadows(word))
	}

	fn word_unless(&self, name: &str, shadows: &dyn Fn(&str) -> bool) -> Option<&'static str> {
		canonical_word(name).filter(|word| !shadows(name) && !shadows(word))
	}

	/// The library word a call of `name` with that many arguments (receiver included) means; shadowing is per arity:
	/// `first(x) := …` leaves `first(xs, 2)` to the library (notes/dispatch.md); a prelude word answers to its own
	/// name too (`round_to(x, 2)` as `round(x, 2)` does, card lib-math)
	fn library_word_for(&self, name: &str, argument_count: usize) -> Option<&'static str> {
		self.word_for_unless(name, argument_count, &|word| self.shadows(word))
	}

	/// The library word `x.name(…)` calls: only the program's functions hide it (Lowering::functions)
	fn method_word_for(&self, name: &str, argument_count: usize) -> Option<&'static str> {
		self.word_for_unless(name, argument_count, &|word| self.functions.contains(word))
	}

	fn word_for_unless(
		&self,
		name: &str,
		argument_count: usize,
		shadows: &dyn Fn(&str) -> bool,
	) -> Option<&'static str> {
		let variant = arity_variant(name, argument_count);
		let prelude_word = || crate::modules::prelude_word(name);
		if self.program.function_takes(name, argument_count) == Some(false) && !self.program.binds(name) {
			return variant.or_else(|| canonical_word(name)).or_else(prelude_word);
		}
		let unshadowed = || prelude_word().filter(|_| !shadows(name));
		variant.filter(|_| !shadows(name)).or_else(|| self.word_unless(name, shadows)).or_else(unshadowed)
	}

	/// `word(x, args)` and `word x args`
	fn word_call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		if let Some(membership) = self.membership(items) {
			return Some(membership);
		}
		if let Some(raised) = raise_call(items, &self.shadowed.borrow(), &self.error_classes) {
			return Some(raised);
		}
		let head = items.first()?.symbol_name()?;
		// `log(x, base)` is ln(x)/ln(base): libm's log takes one argument (unless the program defines its own log)
		if let (LOG_WORD, [_, value, base]) = (head, items) {
			if self.shadows(LOG_WORD) {
				return None;
			}
			if call_name(items, bracket, separator).is_some() {
				return Some(log_to_base(&items[0], value, base));
			}
		}
		// `sin x`, `trunc x`: a library word of one typed value applies to the value after it, as `sin(x)` does, wherever
		// it stands (card sin-cos); a prelude word (`norm v`) is the prelude's function, called by its own pass
		if let ([function, argument], Bracket::None, Separator::Space) = (items, bracket, separator) {
			let is_prelude_word = crate::modules::prelude_word_arity(head).is_some();
			if crate::lowering::library_signatures::typed_first_arity(head) == Some(1)
				&& !is_prelude_word
				&& !self.shadows(head)
			{
				// a synonym calls its library word: `sorted xs` is sort(xs)
				let word = canonical_spelling(head);
				let function = if word == head { function.clone() } else { symbol(word) };
				// the call lowered as if written so: an expanded word (`sum xs`) expands
				let call = word_call(&function, argument, vec![]);
				let Node::List(call_items, call_bracket, call_separator) = &call else { unreachable!("a call") };
				return Some(self.word_call(call_items, call_bracket, call_separator).unwrap_or(call));
			}
		}
		// `module_source("prelude")`: a standard module's text, written out at compile time, so it is read from anywhere
		if let (MODULE_SOURCE_WORD, [_, name]) = (head, items) {
			if let Some(source) = name.text_value().as_deref().and_then(crate::modules::std_module) {
				return Some(Node::Text(source.to_string()));
			}
		}
		// `number(t)` is `t as number`: a text known only at run time is the number it spells, decided at run time
		if let (NUMBER_WORD, [_, value]) = (head, items) {
			if !self.shadows(NUMBER_WORD) && call_name(items, bracket, separator).is_some() {
				return Some(key(value.clone(), Op::As, symbol(NUMBER_WORD)));
			}
		}
		// `first sorted xs` has one argument: a library word after it is its argument's prefix
		let prefixes_its_argument = *bracket == Bracket::None
			&& items.get(1).is_some_and(|next| self.library_word(&next.drop_meta().name()).is_some());
		let argument_count = if prefixes_its_argument { 1 } else { items.len() - 1 };
		let word = self.library_word_for(head, argument_count)?;
		// `sorted "listen" == sorted "silent"`: the word takes the operand of the comparison, as a defined function of `it` does
		if let ([_, argument], Bracket::None, Separator::Space) = (items, bracket, separator) {
			if is_comparison(argument) {
				return Some(with_first_comparand(argument.clone(), &|operand| {
					self.call(word, &items[0], vec![operand], true)
				}));
			}
		}
		let is_call = call_name(items, bracket, separator).is_some();
		// `(sort xs)` is `sort xs` in parentheses
		let is_prefix = is_words_call(bracket, separator) && items.len() > 1;
		// `[sum xs]`: a word with exactly its arguments in a list is its one computed element
		// (a bare word that names no variable keeps the list data: `[first last]`)
		let is_value = |argument: &Node| argument.symbol_name().is_none_or(|name| self.shadows(name));
		let is_element = *bracket == Bracket::Square
			&& *separator == Separator::Space
			&& items.len() == arity(word) + 1
			&& items[1..].iter().all(is_value);
		if is_element && !is_call {
			let element = self.call(word, &items[0], items[1..].to_vec(), false);
			return Some(Node::List(vec![element], Bracket::Square, Separator::None));
		}
		if !is_call && !is_prefix {
			return None;
		}
		Some(in_block_as_written(bracket, self.call(word, &items[0], items[1..].to_vec(), is_prefix && !is_call)))
	}

	/// `for k in m` walks the keys of a map, as in Python; `for k, v in m` and `for (k, v) in m` bind each value too:
	/// `for k in map_keys(m) {v = m[k]; …}`
	fn for_over_map(&self, items: &[Node]) -> Option<Node> {
		let [keyword, names, in_word, rest @ ..] = items else {
			return None;
		};
		if !keyword.is_symbol(FOR_WORD) || !in_word.is_symbol(IN_WORD) {
			return None;
		}
		let (map, body) = crate::lowering::control::for_loop::iterable_and_body(rest)?;
		self.object_literal(&map)?;
		let (key, value) = match names.drop_meta() {
			Node::Symbol(_) => (names.clone(), None),
			Node::List(parts, Bracket::Round | Bracket::None, _) => match parts.as_slice() {
				[key, value] if [key, value].iter().all(|name| name.symbol_name().is_some()) => {
					(key.clone(), Some(value.clone()))
				}
				_ => return None,
			},
			_ => return None,
		};
		let body = match value {
			None => body,
			Some(value) => {
				let lookup = Node::Key(
					Box::new(value),
					Op::Assign,
					Box::new(crate::warp_parser::subscript(map.clone(), key.clone())),
				);
				Node::List([vec![lookup], body.body_statements()].concat(), Bracket::Curly, Separator::Semicolon)
			}
		};
		let keys = self.call(MAP_KEYS, keyword, vec![map], false);
		Some(Node::List(vec![keyword.clone(), key, in_word.clone(), keys, body], Bracket::None, Separator::Space))
	}

	/// `x in xs`: yes/no whether a map has the key x or a list the element x (decision in-bool; its position is
	/// `xs.index_of(x)`); `byte in t` and `"äb" in bytes` count text units. `xs contains x`, `xs has x`,
	/// `xs includes x`: the method `xs.contains(x)` written between its two values, unless the program defines the word
	fn membership(&self, items: &[Node]) -> Option<Node> {
		let [left, word, right] = items else {
			return None;
		};
		// `#bytes in t`, `number of chars in t` (chars.number) count by a unit, a symbol; the text "chars" and a list
		// ["bytes"] are looked up like any value
		let names_unit = |side: &Node| {
			let unit = match side.drop_meta() {
				Node::Key(_, Op::Hash, unit) | Node::Key(unit, Op::Dot, _) => unit.drop_meta(),
				other => other,
			};
			// a variable named so (`chars = ["a"]; "a" in chars`) is that variable
			unit.symbol_name().filter(|name| !self.shadows(name)).and_then(crate::analyzer::text_unit).is_some()
		};
		let (collection, element) = match word.symbol_name()? {
			IN_WORD if names_unit(left) || names_unit(right) => return None,
			IN_WORD => (right, left),
			name if self.library_word_for(name, 2) == Some(COLLECTION_CONTAINS) => (left, right),
			_ => return None,
		};
		Some(self.call(COLLECTION_CONTAINS, word, vec![collection.clone(), element.clone()], false))
	}

	/// `x.word` and `x.word(args)`, one lookup (notes/dispatch.md "One lookup for every word"): a member of the
	/// receiver's own wins over what the word means for any value (a program's function, a module's or the library's
	/// word); `x.word` is `x.word()`. Known at compile time it is chosen there, else at run time; an unknown word on a
	/// value is an error
	fn method_call(&self, receiver: &Node, method: &Node) -> Option<Node> {
		// `pair.0` is the first item of a tuple or list, counted from 0 like `pair[0]`
		if let Node::Number(crate::extensions::numbers::Number::Int(_)) = method.drop_meta() {
			return Some(crate::warp_parser::subscript(receiver.clone(), method.clone()));
		}
		let (word_node, name, arguments) = method_parts(method)?;
		// `c.name` of a parameter `c: function`: what the function value answers at run time
		let is_function_parameter = receiver
			.symbol_name()
			.is_some_and(|parameter| self.function_parameters.borrow().iter().any(|p| p == parameter));
		let is_function_word = || crate::wasm_emitter::functions::function_words().any(|(word, _)| word == name);
		if is_function_parameter && arguments.is_empty() && is_function_word() {
			return Some(field_lookup(receiver, name, word_node));
		}
		if receiver.symbol_name() == Some(DICT_WORD) && name == FROMKEYS {
			return Some(self.call(FROMKEYS, word_node, arguments, false));
		}
		// `x.chars`, `x.chars()`, `chars x` and `chars(x)` are all the list (user 2026-10-14, card abc-chars)
		let is_bare = !matches!(method.drop_meta(), Node::List(..));
		let meaning = self.word_meaning(receiver, name, word_node, &arguments, is_bare);
		let field = || Some(field_lookup(receiver, name, word_node));
		let own_member = self.own_member(receiver, name, &arguments);
		// `m.size` of a map read at run time with an entry size reads the entry, else counts (card map-read); only where
		// the program writes such an entry, as the check makes the count no longer known to be an Int (`xs.length - 1`).
		// `number of bytes in t` reads `bytes.number`: the unit, no variable
		let is_unit =
			receiver.symbol_name().filter(|unit| !self.shadows(unit)).and_then(crate::analyzer::text_unit).is_some();
		if is_bare
			&& !is_unit
			&& matches!(own_member, OwnMember::Maybe)
			&& self.entry_names.contains(name)
			&& crate::analyzer::is_counting_property(name)
		{
			// `x.number` stays for the emitter's counter (counting_getter): `number(x)` would cast x
			return Some(self.member_or(receiver, name, word_node, key(receiver.clone(), Op::Dot, method.clone())));
		}
		// `p.center`, `s.words` of another module's word on what may be an object: its field, as before the word was
		// loaded; a parameter or a user function's result may be of the word's type module, chosen at run time
		let is_module_word = is_bare && is_loaded_module_word(name) && self.method_word_for(name, 1).is_none();
		let may_be_object = match own_member {
			OwnMember::Maybe => true,
			OwnMember::Not => self.is_object(receiver, name),
			OwnMember::Has | OwnMember::Lacks => false,
		};
		if is_module_word && may_be_object {
			let is_generic = self.is_parameter(receiver)
				|| receiver.symbol_name().is_some_and(|variable| self.call_results.contains(variable));
			return match self.type_module_of(name).filter(|_| is_generic) {
				Some(_) => Some(self.member_or(receiver, name, word_node, word_call(word_node, receiver, vec![]))),
				None => field(),
			};
		}
		match (own_member, meaning) {
			(OwnMember::Has, _) => field(),
			(OwnMember::Maybe, WordMeaning::Call { call, typed: false }) => {
				Some(self.member_or(receiver, name, word_node, call))
			}
			(_, WordMeaning::Call { call, .. }) => Some(call),
			(_, WordMeaning::Kept) => None,
			(OwnMember::Maybe | OwnMember::Lacks, _) | (OwnMember::Not, WordMeaning::FieldRead) => field(),
			(OwnMember::Not, WordMeaning::Undefined) => {
				let is_value = matches!(
					receiver.drop_meta(),
					Node::Symbol(_)
						| Node::Text(_) | Node::Char(_)
						| Node::Number(_) | Node::List(_, Bracket::Square, _)
				);
				let call = key(receiver.clone(), Op::Dot, method.clone());
				is_value.then(|| self.context.undefined_function_diagnostic(&call, name).into_error())
			}
		}
	}

	/// Whether the receiver has a member `name` of its own (level 1 of the one lookup): an entry of the object literal
	/// it holds, a field of its declared class, or, for a value known only at run time, maybe
	fn own_member(&self, receiver: &Node, name: &str, arguments: &[Node]) -> OwnMember {
		// a member is read without arguments: `p.first`, `p.first()`
		if !arguments.is_empty() {
			return OwnMember::Not;
		}
		let literal = self.object_literal(receiver);
		let literal_has_it = (literal.as_ref().and_then(object_entries))
			.is_some_and(|entries| entries.iter().any(|(field, _)| field == name));
		// `p.name` inside the function `name(p)`: a call would recurse with the same argument forever
		if literal_has_it
			|| self.instances.is_instance_field(receiver, name)
			|| self.reads_own_parameter_field(receiver, name)
		{
			return OwnMember::Has;
		}
		// a sum type's value holds a variant known only at run time: `t.count` of `t: T` may be C(count)'s field
		let is_known = literal.is_some() || self.instances.shape(receiver).is_some();
		if is_known && !self.instances.is_sum_typed(receiver) {
			return OwnMember::Lacks;
		}
		let is_element = matches!(receiver.drop_meta(), Node::Key(_, Op::Hash, _));
		// a parameter named like a variable assigned a plain value elsewhere holds any value
		let is_plain = receiver.symbol_name().is_some_and(|variable| self.plain.contains(variable))
			&& !self.is_parameter(receiver);
		match !is_plain && (is_element || self.is_object(receiver, name) || self.may_hold_object(receiver)) {
			true => OwnMember::Maybe,
			false => OwnMember::Not,
		}
	}

	/// What `x.word(args)` means for any value, below a member of x's own: the library word, the program's function
	/// taking x first, a conversion, a number word, `type`, or a swizzle
	fn word_meaning(
		&self,
		receiver: &Node,
		name: &str,
		word_node: &Node,
		arguments: &[Node],
		is_bare: bool,
	) -> WordMeaning {
		let has_arguments = !arguments.is_empty();
		let call = |call: Node| WordMeaning::Call { call, typed: false };
		// a counting word without parentheses stays the count the emitter reads (`xs.count`)
		if let Some(word) = self
			.method_word_for(name, arguments.len() + 1)
			.filter(|_| !is_bare || counting_method(name, &self.context).is_none())
		{
			let typed = !reads_entry_first(name, word);
			let call = self.call(word, word_node, [vec![receiver.clone()], arguments.to_vec()].concat(), false);
			return WordMeaning::Call { call, typed };
		}
		// x.word is x.word(): a bare `xs.pop` changes xs as `xs.pop()` does (card xs-xs)
		if is_bare && is_list_mutating_method(name) {
			let call = key(receiver.clone(), Op::Dot, crate::lowering::nodes::call(name, vec![]));
			return WordMeaning::Call { call, typed: true };
		}
		// `p.center`, `s.items` of another module's word: no call, as before the word was loaded
		let is_other_modules_word = is_bare && is_loaded_module_word(name) && self.type_module_of(name).is_none();
		// the uniform call: `x.f(args)` is `f(x, args)` when the program's f takes args + 1 values
		let is_users =
			|name: &str| self.context.user_functions.get(name).is_some_and(|function| !function.params.is_empty());
		if !is_other_modules_word && is_users(name) && self.program.takes_method_call(name, arguments.len()) {
			return call(word_call(word_node, receiver, arguments.to_vec()));
		}
		// `x.string`, `t.int`, `x.float()`, `2.double()`: the conversion `string(x)`, as written in call form; uncalled
		// only the common conversions (not `e.text`, a common field); a counting word stays a count (`xs.number()`)
		let is_type_word_call = !is_bare
			&& crate::analyzer::type_word_kind(name).is_some()
			&& counting_method(name, &self.context).is_none();
		let is_conversion = CONVERSION_METHODS.contains(&name) || is_type_word_call;
		if is_conversion && !has_arguments {
			return call(word_call(word_node, receiver, vec![]));
		}
		// a program function of other arities: its call names what is missing or extra
		if !is_other_modules_word && is_users(name) {
			return call(word_call(word_node, receiver, arguments.to_vec()));
		}
		if self.is_known_method(receiver, name) {
			return WordMeaning::Kept;
		}
		if has_arguments {
			return WordMeaning::Undefined;
		}
		// `x.round`, `x.sqrt`: the number word applied to x, as `x.round()` and `sqrt x` are (x.word is word(x))
		if let Some(operation) = prefix_operator_method(receiver, name) {
			return call(operation);
		}
		// `x.type`, `x.type()`: `type(x)`
		if crate::wasm_emitter::ROUNDING_FUNCTIONS.contains(&name) || name == crate::type_tests::TYPE_WORD {
			return call(word_call(word_node, receiver, vec![]));
		}
		// `p.x`, `p.zy` of a list: its components when it holds 2 to 4 numbers, read as a field (decision S1)
		match crate::words::swizzle_components(name) {
			Some(_) => WordMeaning::FieldRead,
			None => WordMeaning::Undefined,
		}
	}

	/// The type module (lib/list.warp, text.warp) of a loaded word of one parameter, which `x.word` calls
	fn type_module_of(&self, name: &str) -> Option<&'static str> {
		let takes_one = self.context.user_functions.get(name).is_some_and(|function| function.params.len() == 1);
		type_module_defining(name).filter(|_| takes_one)
	}

	/// A word a value answers other than by a field: a count, a list change, a user function, or `f.effects` of a
	/// function (not of a variable, a parameter or an object: `b.effects` of those reads the field), the query effects.rs
	/// answers
	fn is_known_method(&self, receiver: &Node, name: &str) -> bool {
		let is_effects_query = name == crate::analyzer::effects::EFFECTS_WORD
			&& !self.is_parameter(receiver)
			&& receiver
				.symbol_name()
				.is_some_and(|function| !self.plain.contains(function) && !self.objects.contains_key(function));
		counting_method(name, &self.context).is_some()
			|| is_list_mutating_method(name)
			|| self.context.user_functions.contains_key(name)
			|| is_effects_query
	}

	/// A receiver known to be an object: a field lookup, a parameter, a call's result, or one of a declared field name
	fn is_object(&self, receiver: &Node, name: &str) -> bool {
		let is_call_result = receiver.symbol_name().is_some_and(|variable| self.call_results.contains(variable));
		is_field_lookup(receiver)
			|| self.is_parameter(receiver)
			|| is_call_result
			|| self.instances.is_declared_field(name)
	}

	/// `x.word` of an x that may have a member `word` of its own, known only at run time: `if x has it then x["word"]
	/// else <word_call>`. A map has it as a key (a one-entry map in a list, `[{first: 2}]`, is held as a key, which has
	/// that key too), an instance of a declared class with that field; a map's own words (`m.count`) still answer on
	/// maps without such a key
	fn member_or(&self, receiver: &Node, name: &str, word_node: &Node, word_call: Node) -> Node {
		if !is_pure_read(receiver) {
			return word_call;
		}
		let is_type = |type_word: &str| parens(vec![symbol(crate::words::IS_TYPE), receiver.clone(), text(type_word)]);
		let is_map = key(is_type(crate::words::MAP_WORD), Op::Or, is_type(&crate::Kind::Key.to_string()));
		let has_key = self.call(COLLECTION_CONTAINS, word_node, vec![receiver.clone(), text(name)], false);
		let has_member = (self.instances.classes_with_field(name).into_iter())
			.fold(key(parens(vec![is_map]), Op::And, has_key), |test, class| key(is_type(&class), Op::Or, test));
		if_then_else(has_member, field_lookup(receiver, name, word_node), word_call)
	}

	fn may_hold_object(&self, receiver: &Node) -> bool {
		// a variable of unknown value (`o` of `for o in people.filter(…)`), or an element of one (`xs[1]`), may hold an
		// object: its field is looked up at run time
		// a parenthesized receiver is what it holds: `(x#2).name` is `x#2.name` (card name-quoted); `f()` stays a call
		let inner = match receiver.drop_meta() {
			Node::List(items, Bracket::Round, _) if items.len() == 1 && items[0].symbol_name().is_none() => {
				items[0].drop_meta()
			}
			other => other,
		};
		let variable = match inner {
			Node::Key(list, Op::Hash, _) => list.drop_meta(),
			other => other,
		};
		match variable {
			Node::Symbol(variable) => {
				!self.plain.contains(variable) && !self.context.user_functions.contains_key(variable)
			}
			// the value of a call: `f().name`, `lib.stats_of(t).words` (foreign_call)
			Node::List(items, Bracket::Round, Separator::None) if word_headed(items).is_some() => true,
			// a parenthesized object `({a:1}).a`, or quoted code `(data n*1).op` (node_api.rs)
			Node::List(_, Bracket::Curly, _) => true,
			_ if crate::blocks::quoted_data(variable).is_some() => true,
			// a time the run computes: `(now + 1 hour).hour`, `(x + 1 hour).hour` of `x = now` (cards field-grouped,
			// variable-time)
			_ if crate::time::holds_time(inner, &self.times_here()) => true,
			// an element of a computed list: `users.filter(…)#1.name`
			_ => matches!(inner, Node::Key(_, Op::Hash, _)) && !is_plain_literal(variable),
		}
	}

	/// The call of `word` with the receiver and its arguments; arguments are the items after the word for the prefix form
	fn call(&self, word: &'static str, head: &Node, arguments: Vec<Node>, is_prefix: bool) -> Node {
		let mut arguments = if is_prefix { self.merged_prefix_arguments(word, arguments) } else { arguments };
		let wanted = arity(word);
		if arguments.len() < wanted && arguments.len() >= fewest_arguments(word) {
			while arguments.len() < wanted {
				arguments.push(left_out_argument(word, arguments.len()));
			}
		}
		// objects are references (P200): `x.copy()` is a new deep copy, `x.copy(shallow: true)` a shallow one (P205)
		if let (COPY_WORD, [receiver], shallow) | (COPY_WORD, [receiver, shallow], _) =
			(word, arguments.as_slice(), &Node::False)
		{
			return copy_call(receiver.clone(), shallow_flag(shallow));
		}
		// a text builtin's own arity check names its values (`trim takes 1 value, got 2`)
		let checks_itself = crate::wasm_emitter::text::text_builtins::is_text_builtin(word);
		// a prelude parameter with a default may be left out: its definition fills it in
		let fewest = wanted - crate::modules::prelude_word_defaults(word);
		if !(fewest..=wanted).contains(&arguments.len()) && !checks_itself {
			let plural = if wanted == 1 { "" } else { "s" };
			let counts = if fewest == wanted { wanted.to_string() } else { format!("{fewest} to {wanted}") };
			let call = parens([vec![head.clone()], arguments.clone()].concat());
			return Diagnostic::at(&call, format!("{word} takes {counts} argument{plural}, got {}", arguments.len()))
				.into_error();
		}
		match EXPANDED_WORDS.iter().find(|(name, _, _)| *name == word) {
			Some((_, _, template)) if word == SUM_WORD => dispatched_sum(self.expanded(template, arguments)),
			Some((_, _, template)) => self.expanded(template, arguments),
			// a prelude word calls its definition from lib/prelude.warp (modules::prelude_name)
			None if let Some(qualified) = crate::modules::prelude_name(word) => {
				call(&qualified, crate::modules::prelude_arguments(word, arguments))
			}
			None => {
				// else the written spelling's position: `warn "x"` errs where warn is
				let name = if head.is_symbol(word) { head.clone() } else { symbol(word).with_meta_of(head) };
				parens([vec![name], arguments].concat())
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
		let heads_with_word = rest[0].symbol_name().is_some_and(|name| self.library_word(name).is_some());
		let phrase = heads_with_word.then(|| self.word_call(rest, &Bracket::None, &Separator::Space)).flatten();
		let phrase = phrase.unwrap_or_else(|| crate::lowering::ambiguous_forms::spaced_phrase(rest.to_vec()));
		[kept.to_vec(), vec![phrase]].concat()
	}

	/// The template of an expanded word with the receiver (held once in a temporary) and the further arguments in place
	fn expanded(&self, template: &str, arguments: Vec<Node>) -> Node {
		let program = named_apart(
			parse_template(
				&format!("({TEMPORARY}={RECEIVER_PLACEHOLDER}; {template})"),
				arguments.first().unwrap_or(&Node::Empty),
			),
			HIDDEN_PREFIX,
			WORD_BASE,
			self.next_temporary(),
		);
		arguments.iter().enumerate().fold(program, |program, (index, argument)| {
			let placeholder = if index == 0 {
				RECEIVER_PLACEHOLDER.to_string()
			} else {
				format!("{RECEIVER_PLACEHOLDER}_{}", index + 1)
			};
			substitute(program, &placeholder, argument)
		})
	}
}

/// `(tmp = xs; loop)` → `(tmp = xs; list_sum(tmp, loop))`
fn dispatched_sum(expanded: Node) -> Node {
	let Node::List(mut items, bracket, separator) = expanded.drop_meta().clone() else {
		return expanded;
	};
	let [assignment, sum_loop] = items.as_mut_slice() else {
		return Node::List(items, bracket, separator);
	};
	let Node::Key(temporary, Op::Assign, _) = assignment.drop_meta() else {
		return Node::List(items, bracket, separator);
	};
	let list = temporary.as_ref().clone();
	*sum_loop = call(LIST_SUM, vec![list, sum_loop.clone()]);
	Node::List(items, bracket, separator)
}

/// `a == b`, `a < b`, `a and b`: where a library word's argument stops
pub(crate) fn is_comparison(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, op, _) if op.is_comparison() || op.is_logical())
}

/// The leftmost operand of a comparison chain, replaced: `"a" == b and c` → `f("a") == b and c`
pub(crate) fn with_first_comparand(node: Node, replace: &dyn Fn(Node) -> Node) -> Node {
	match node {
		Node::Key(left, op, right) if op.is_comparison() || op.is_logical() => {
			Node::Key(Box::new(with_first_comparand(*left, replace)), op, right)
		}
		Node::Meta { node, data } if is_comparison(&node) => {
			Node::Meta { node: Box::new(with_first_comparand(*node, replace)), data }
		}
		operand => replace(operand),
	}
}

/// A zero written with a decimal point, `0.0`
fn is_float_zero(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Number(crate::extensions::numbers::Number::Float(value)) if *value == 0.0)
}

/// The name and the untyped parameters of `def f(a, b) {…}` (any function keyword): the head `f (a, b)` holds them
fn keyword_definition(items: &[Node]) -> Option<(Option<String>, Vec<String>)> {
	let (keyword, head) = match items {
		[keyword, head, _body] => (keyword, head),
		// `def ((f (m)) {…})`: the head and the body as one group
		[keyword, definition] => match definition.drop_meta() {
			Node::List(parts, Bracket::Round, _) if parts.len() == 2 => (keyword, &parts[0]),
			_ => return None,
		},
		_ => return None,
	};
	let Node::List(head_items, Bracket::Round, _) = head.drop_meta() else {
		return None;
	};
	if !super::nodes::is_function_keyword(keyword) {
		return None;
	}
	let parameters = head_items.iter().skip(1).flat_map(|item| match item.drop_meta() {
		Node::List(group, Bracket::Round, _) => group.clone(),
		other => vec![other.clone()],
	});
	let parameters = parameters
		.filter_map(|parameter| match parameter.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			Node::Key(name, Op::Assign, _) => Some(name.name()),
			_ => None,
		})
		.collect();
	Some((head_items.first().map(Node::name), parameters))
}
