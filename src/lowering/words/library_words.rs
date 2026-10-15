//! The basic text and list words (`upper`, `first`, `sum`, `split` …) in their three spellings: `x.word`, `x.word(args)`,
//! `word(x, args)` and `word x args` all become the call `word(x, args)`.
//! Words that need no runtime function are expanded to source here (`first`, `last`, `sum`); the others are calls that the
//! emitter resolves (`RUNTIME_WORDS`). A user function or variable of the same name wins.
//!
//! An unknown `.word` after a name, a text or a list is a loud error (`undefined function: word`), never silent data.

use super::nodes::{call, children_rewritten, if_then_else, key, Counter};
use crate::analyzer::{call_name, counting_method};
use crate::compiler::context::Context;
use crate::errors::diagnostic::Diagnostic;
use crate::list_mutations::is_list_mutating_method;
use crate::node::{parens, symbol, text, Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::{parse, ASSERT_MARKER, TRY_MARKER};
use crate::wasm_emitter::{CAUGHT_ERROR, RAN_WITHOUT_ERROR};
use crate::words::COPY_WORD;
use crate::words::COUNT_WORD;
use crate::words::FLOAT_WORD;
use crate::words::FOR_WORD;
use crate::words::IN_WORD;
use crate::words::LIST_WORD;
use crate::words::LOG_WORD;
use crate::words::ORD_WORD;
use crate::words::SPLIT_WORD;
use crate::words::{CODEPOINT, MAP_KEYS, RAISE_WORD, SLICE, SUM_WORD};
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
/// The name `catch e` binds, in lower_try_binding's template
/// The modules named after a type, whose words `x.word` calls on a value of it (lib/list.warp, text.warp)
const TYPE_MODULES: [&str; 2] = ["list", "text"];
const CAUGHT_BINDING_PLACEHOLDER: &str = "caught_binding";
/// The got-it topic of `square 3 + square(4)` (ask_braceless_argument)
const BRACELESS_ARGUMENT_TOPIC: &str = "braceless-call-operator";
const COUNT_HAYSTACK: &str = "counted_haystack";
const COUNT_NEEDLE: &str = "counted_needle";
const COUNT_ITEM_TEMPLATE: &str = "count(filter(counted_haystack, counted_item => counted_item == counted_needle))";
const COUNT_SUBSTRING_TEMPLATE: &str = "count(split(counted_haystack, counted_needle)) - 1";

/// Canonical word and the spellings that mean it
const SYNONYMS: [(&str, &[&str]); 31] = [
	(crate::wasm_emitter::text::text_builtins::WARNING, &["warn"]),
	// Python's tuple(xs) (card python-stdlib)
	(LIST_WORD, &["tuple"]),
	(MAP_KEYS, &["keys"]),
	(MAP_VALUES, &["values"]),
	(MAP_ENTRIES, &[]),
	(COLLECTION_CONTAINS, &["contains", "has", "includes"]),
	(MAP_GET_OR, &["get"]),
	// P35 default: `xs.index_of(x)` is the 1-based position (0 when absent)
	(COLLECTION_POSITION, &["index_of"]),
	("chars", &[]),
	("upper", &["uppercase"]),
	("lower", &["lowercase"]),
	// the text builtin trim (text_builtins.rs), Python's strip
	("trim", &["strip"]),
	("reverse", &["reversed"]),
	("sort", &["sorted"]),
	("split", &[]),
	("join", &[]),
	("first", &[]),
	("last", &[]),
	(SLICE, &[]),
	(COPY_WORD, &["clone"]),
	("replace", &[]),
	(ORD_WORD, &["ordinal", CODEPOINT]),
	(IS_DIGIT, &["isdigit"]),
	("mean", &["average"]),
	(IS_ALPHA, &["is_letter", "isalpha"]),
	("is_alphanumeric", &["is_alnum", "isalnum"]),
	("is_space", &["is_whitespace", "isspace"]),
	("nearest", &[]),
	// lib/prelude.warp's map of [key value] pairs, Python's dict(pairs)
	(DICT_WORD, &[]),
	(FROMKEYS, &[]),
	(crate::words::UNWRAP, &[]),
];
/// `list_sum(list, loop)`: the sum of a list variable as one operation the emitter dispatches (wasm_emitter/list_dispatch.rs):
/// a typed list sums its array, any other list runs the loop `sum` always lowered to
pub const LIST_SUM: &str = "list_sum";

/// Words the emitter implements as runtime functions, with the number of arguments including the receiver
pub const RUNTIME_WORDS: [(&str, usize); 20] = [
	(crate::wasm_emitter::host::wasi_emitter::WASI_ENVIRONMENT, 0),
	(ORD_WORD, 1),
	("upper", 1),
	("lower", 1),
	("reverse", 1),
	("sort", 1),
	("split", 2),
	("join", 2),
	("chars", 1),
	(FIELD_WITH, 3),
	(MAP_KEYS, 1),
	(MAP_VALUES, 1),
	(MAP_ENTRIES, 1),
	(COLLECTION_CONTAINS, 2),
	(COLLECTION_POSITION, 2),
	(MAP_GET_OR, 3),
	(SLICE, 3),
	(MAP_WITHOUT, 2),
	(INSTANCE_COPY, 2),
	(crate::one_entry_maps::MAP_OF_ENTRY, 1),
];
/// Trailing arguments a word may leave out, passed as ø: `m.get(k)` is ø for a missing key, `s.slice(2)` slices to the end;
/// `s.split` splits at spaces (left_out_argument)
const OPTIONAL_ARGUMENTS: [(&str, usize); 3] = [(MAP_GET_OR, 1), (SLICE, 1), (SPLIT_WORD, 1)];
const SPLIT_SEPARATOR: &str = " ";
/// What an optional argument left out is: the space `s.split` splits at, else ø
fn left_out_argument(word: &str) -> Node {
	match word {
		SPLIT_WORD => text(SPLIT_SEPARATOR),
		_ => Node::Empty,
	}
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
/// (first, last, round_to, replace, is_digit …) are lib/prelude.warp's; these three the emitter dispatches on
const EXPANDED_WORDS: [(&str, usize, &str); 3] = [
	// Python's `list(x)`: the list itself, a text's characters
	(LIST_WORD, 1, "hidden_value as list"),
	(
		SUM_WORD,
		1,
		"(hidden_sum=0; for hidden_item in hidden_value {hidden_sum = hidden_sum + hidden_item}; hidden_sum)",
	),
	// `x!` (mutation.rs): the value, a loud error when it is ø; an Error value stays that Error
	(crate::words::UNWRAP, 1, "if hidden_value == ø then error \"unwrapped ø\" else hidden_value"),
];
const IS_DIGIT: &str = "is_digit";
const DICT_WORD: &str = "dict";
/// lib/prelude.warp's fromkeys(ks, value), also called on the class as Python's `dict.fromkeys(ks, value)`
const FROMKEYS: &str = "fromkeys";
/// Conversions a method may name, `x.string` being `string(x)`
const CONVERSION_METHODS: [&str; 4] = ["string", "str", "int", "float"];
const LIST_CONSTRUCTORS: [&str; 4] = ["listOf", "mutableListOf", "arrayOf", "arrayListOf"];
/// Words that mean another word with that many arguments (receiver included): `round(x, 3)`, `x.round(3)` is x rounded
/// to 3 digits after the point (`round(x)` stays the builtin); `xs.first(3)`, `first 3 of xs` the first 3 items,
/// `last(xs, 2)` the last 2 (lib/prelude.warp)
const ARITY_VARIANTS: [(&str, usize, &str); 3] =
	[("round", 2, "round_to"), ("first", 2, "first_items"), ("last", 2, "last_items")];
/// Methods a later pass lowers to prelude words: `x.pop(k)` (and `delete x[k]`, mutation::lower_deletes) to popped_item
/// and popped_rest, `x.clear()` to cleared (list_mutations removed_key, cleared_collection)
const POPPED_WORDS: &[&str] = &["popped_item", "popped_rest"];
const LOWERED_TO_PRELUDE: [(&str, &[&str]); 3] =
	[("pop", POPPED_WORDS), ("delete", POPPED_WORDS), ("clear", &["cleared"])];
const IS_ALPHA: &str = "is_alpha";
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

/// Library words whose result is always a text, and those whose result is always a list
const TEXT_RESULT_WORDS: [&str; 5] =
	["upper", "lower", "trim", "join", crate::wasm_emitter::host::wasi_emitter::WASI_ENVIRONMENT];
const LIST_RESULT_WORDS: [&str; 8] = [
	"chars",
	"sort",
	"split",
	MAP_KEYS,
	MAP_VALUES,
	MAP_ENTRIES,
	MAP_WITHOUT,
	crate::wasm_emitter::lists::list_ops::LIST_EXTEND,
];

pub fn result_kind(word: &str) -> Option<crate::type_kinds::Kind> {
	use crate::type_kinds::Kind;
	let word = canonical_spelling(word);
	if TEXT_RESULT_WORDS.contains(&word) {
		Some(Kind::Text)
	} else {
		LIST_RESULT_WORDS.contains(&word).then_some(Kind::List)
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
	word.name() == FIELD_WITH && matches!(map.drop_meta(), Node::Symbol(map) if map == name)
}

pub fn is_runtime_word(name: &str) -> bool {
	RUNTIME_WORDS.iter().any(|(word, _)| *word == name)
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

/// A library word of one argument takes the name after it as that argument inside an operand, as a defined function
/// does: `word == reverse word` compares with `reverse(word)`, `1 == first xs` with `first(xs)`. A type word stays a
/// type: `x: list int`
pub fn is_operand_word(name: &str) -> bool {
	canonical_word(name).is_some_and(|word| arity(word) == 1 && !crate::type_tests::is_builtin_type_word(word))
}

/// The spelling word lists name a library word by (`sorted` → sort); any other name as it is
pub fn canonical_spelling(name: &str) -> &str {
	canonical_word(name).unwrap_or(name)
}

fn canonical_word(name: &str) -> Option<&'static str> {
	SYNONYMS
		.iter()
		.find(|(word, synonyms)| *word == name || synonyms.contains(&name))
		.map(|(word, _)| *word)
		.or((name == SUM_WORD).then_some(SUM_WORD))
}

pub(crate) fn arity(word: &str) -> usize {
	let expanded = EXPANDED_WORDS.iter().map(|(name, arity, _)| (*name, *arity));
	let built_in = RUNTIME_WORDS.into_iter().chain(expanded).find(|(name, _)| *name == word).map(|(_, arity)| arity);
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
	crate::type_tests::collect_outer_assigned_names(&node, &mut shadowed);
	let mut assigned = AssignedObjects::default();
	collect_assigned_objects(&node, &mut assigned);

	let plain = assigned.plain.difference(&assigned.rebound).cloned().collect();
	let objects = assigned.objects.into_iter().filter_map(|(name, literal)| Some((name, literal?))).collect();
	let instances = crate::traits::InstanceTypes::of(&node);
	let call_results = call_results(&node, &context);
	Lowering {
		context,
		shadowed: RefCell::new(shadowed),
		objects,
		plain,
		instances,
		parameters: RefCell::new(vec![]),
		defining: RefCell::new(vec![]),
		call_results,
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
				if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(function)) if context.user_functions.contains_key(function)));
			if let (Node::Symbol(name), true) = (target.drop_meta(), calls_user_function) {
				names.insert(name.clone());
			}
		}
	});
	names
}

/// `raise X`, `throw X`, `fail X`, `panic X`, `raise(X)`: the call `raise(X)`; `raise error "m"` raises the message m.
/// A word the program defines (`shadowed`) is its own
fn raise_call(items: &[Node], shadowed: &HashSet<String>) -> Option<Node> {
	let [word, raised] = items else { return None };
	let is_raise_word = |raise: &&str| word.is_symbol(raise) && !shadowed.contains(*raise);
	if !crate::errors::run_time::RAISE_WORDS.iter().any(is_raise_word) {
		return None;
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
		Node::Key(field, op @ (Op::Inc | Op::Dec), _) if matches!(field.drop_meta(), Node::Symbol(_)) => {
			Some((field.as_ref().clone(), *op))
		}
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
	key(place, update, Node::int(1))
}

/// The word of a method and its arguments: `x.word`, `x.word(args)`
fn method_parts(method: &Node) -> Option<(&Node, &String, Vec<Node>)> {
	let (word_node, arguments) = match method.drop_meta() {
		Node::Symbol(_) => (method, vec![]),
		Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => {
			(&items[0], items[1..].to_vec())
		}
		_ => return None,
	};
	let Node::Symbol(name) = word_node.drop_meta() else {
		return None;
	};
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

/// `x.word` of an x that may be an object: `if x is <module type> then word(x) else x["word"]`; a receiver that is no
/// variable reads the field, as it is evaluated once
fn field_or_word(receiver: &Node, word_node: &Node, name: &str, module: &str) -> Node {
	let field = field_lookup(receiver, name, word_node);
	if receiver.symbol_name().is_none() {
		return field;
	}
	let is_of_type = parens(vec![symbol(crate::words::IS_TYPE), receiver.clone(), text(module)]);
	if_then_else(is_of_type, word_call(word_node, receiver, vec![]), field)
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
				let left = self.expand(*left);
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
				let left = self.expand(*left);
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
		let lowered = self.within_function(name, parameters, || self.expand(self.body_statement(head, right)));
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
		let defines_function = matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))));
		match right {
			Node::List(items, Bracket::Curly, Separator::Space)
				if defines_function
					&& items.len() > 1
					&& matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) =>
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
			Node::List(items, bracket, separator)
				if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) =>
			{
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
		canonical_word(name).filter(|word| !self.shadows(name) && !self.shadows(word))
	}

	/// The library word a call of `name` with that many arguments (receiver included) means
	fn library_word_for(&self, name: &str, argument_count: usize) -> Option<&'static str> {
		self.arity_variant(name, argument_count).or_else(|| self.library_word(name))
	}

	/// The word `name` means with that many arguments, unless the program defines `name`
	fn arity_variant(&self, name: &str, argument_count: usize) -> Option<&'static str> {
		let variant = ARITY_VARIANTS
			.iter()
			.find(|(word, count, _)| *word == name && *count == argument_count)
			.map(|(_, _, variant)| *variant);
		variant.filter(|_| !self.shadows(name))
	}

	/// `word(x, args)` and `word x args`
	fn word_call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		if let Some(membership) = self.membership(items) {
			return Some(membership);
		}
		if let Some(raised) = raise_call(items, &self.shadowed.borrow()) {
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
			if self.shadows(LOG_WORD) {
				return None;
			}
			if call_name(items, bracket, separator).is_some() {
				let log_of = |argument: &Node| parens(vec![items[0].clone(), argument.clone()]);
				return Some(key(log_of(value), Op::Div, log_of(base)));
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
		let is_prefix =
			matches!(bracket, Bracket::None | Bracket::Round) && *separator == Separator::Space && items.len() > 1;
		// `[sum xs]`: a word with exactly its arguments in a list is its one computed element
		// (a bare word that names no variable keeps the list data: `[first last]`)
		let is_value = |argument: &Node| !matches!(argument.drop_meta(), Node::Symbol(name) if !self.shadows(name));
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
		Some(self.call(word, &items[0], items[1..].to_vec(), is_prefix && !is_call))
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
				[key, value] if [key, value].iter().all(|name| matches!(name.drop_meta(), Node::Symbol(_))) => {
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

	/// `square 3 + square(4)`, a braceless call of a user function whose argument holds an operator, reads as
	/// square(3 + square(4)); P68 (user): the reading stays, with a got-it warning naming both readings
	fn ask_braceless_argument(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Result<(), Node> {
		let [function, argument] = items else { return Ok(()) };
		let Node::Symbol(name) = function.drop_meta() else {
			return Ok(());
		};
		let Node::Key(left, op, right) = argument.drop_meta() else {
			return Ok(());
		};
		if *bracket != Bracket::None
			|| *separator != Separator::Space
			|| !self.context.user_functions.contains_key(name)
			|| !op.is_arithmetic()
			|| matches!(left.drop_meta(), Node::Empty)
		{
			return Ok(());
		}
		let (left, right) = (call_text(left), call_text(right));
		let whole = format!("{name}({left} {op} {right})");
		let first = format!("{name}({left}) {op} {right}");
		let question = crate::errors::diagnostic::Ask::new(
			BRACELESS_ARGUMENT_TOPIC,
			format!("`{name} {left} {op} {right}` is {whole}, not {first}"),
			vec![
				crate::errors::diagnostic::reading("the whole expression", &whole),
				crate::errors::diagnostic::reading("only the first operand", &first),
			],
			crate::errors::diagnostic::Fallback::Warning,
		)
		.written(&format!("{name} {left} {op} {right}"))
		.at_node(argument);
		crate::errors::diagnostic::ask(&question).map(|_| ())
	}

	/// `x in xs`: yes/no whether a map has the key x or a list the element x (decision in-bool; its position is
	/// `xs.index_of(x)`); `byte in t` and `"äb" in bytes` count text units. `xs contains x`, `xs has x`,
	/// `xs includes x`: the method `xs.contains(x)` written between its two values, unless the program defines the word
	fn membership(&self, items: &[Node]) -> Option<Node> {
		let [left, word, right] = items else {
			return None;
		};
		let names_unit = |side: &Node| match side.drop_meta() {
			Node::Key(_, Op::Hash, unit) => crate::analyzer::text_unit(&unit.name()).is_some(),
			other => crate::analyzer::text_unit(&other.name()).is_some(),
		};
		let (collection, element) = match word.symbol_name()? {
			IN_WORD if names_unit(left) || names_unit(right) => return None,
			IN_WORD => (right, left),
			name if self.library_word_for(name, 2) == Some(COLLECTION_CONTAINS) => (left, right),
			_ => return None,
		};
		Some(self.call(COLLECTION_CONTAINS, word, vec![collection.clone(), element.clone()], false))
	}

	/// `x.word` and `x.word(args)`; an unknown word on a value is an error
	fn method_call(&self, receiver: &Node, method: &Node) -> Option<Node> {
		// `pair.0` is the first item of a tuple or list, counted from 0 like `pair[0]`
		if let Node::Number(crate::extensions::numbers::Number::Int(_)) = method.drop_meta() {
			return Some(crate::warp_parser::subscript(receiver.clone(), method.clone()));
		}
		let (word_node, name, arguments) = method_parts(method)?;
		if receiver.symbol_name() == Some(DICT_WORD) && name == FROMKEYS {
			return Some(self.call(FROMKEYS, word_node, arguments, false));
		}
		let has_arguments = !arguments.is_empty();
		let literal = self.object_literal(receiver);
		let is_field = literal
			.as_ref()
			.and_then(object_entries)
			.is_some_and(|entries| entries.iter().any(|(field, _)| field == name));
		if is_field && !has_arguments {
			return Some(field_lookup(receiver, name, word_node));
		}
		// `x.chars`, `x.chars()`, `chars x` and `chars(x)` are all the list (user 2026-10-14, card abc-chars)
		let is_called = matches!(method.drop_meta(), Node::List(..));
		// `n.first` of a person with a field first reads the field, not the library word first
		if !is_called && self.instances.is_instance_field(receiver, name) {
			return Some(field_lookup(receiver, name, word_node));
		}
		if let Some(word) = self
			.library_word_for(name, arguments.len() + 1)
			.filter(|_| is_called || counting_method(name, &self.context).is_none())
		{
			return Some(self.call(word, word_node, [vec![receiver.clone()], arguments].concat(), false));
		}
		if !is_called && self.reads_declared_field(receiver, name) {
			return Some(field_lookup(receiver, name, word_node));
		}
		// `b.product` of a value that may be an object: its field if it is no value of the word's type, else the type
		// module's word, as `b.product()` calls it (x.word is x.word(), user 2026-10-14)
		// `p.center`, `s.items` of another module's word or one of more parameters: no call, as before the word was loaded
		let is_loaded_word = !is_called && is_loaded_module_word(name);
		let type_module = self.type_module_of(name).filter(|_| is_loaded_word);
		let may_be_object = || self.is_object(receiver, name) || self.may_hold_object(receiver);
		if is_loaded_word && may_be_object() {
			// a parameter's or a user function result's type shows at run time; a loop's element or a foreign
			// result (`r.stats_of(t).words`) reads the field, as a branch of the word would type the value
			let is_generic = self.is_parameter(receiver)
				|| receiver.symbol_name().is_some_and(|variable| self.call_results.contains(variable));
			return Some(match type_module.filter(|_| is_generic) {
				Some(module) => field_or_word(receiver, word_node, name, module),
				None => field_lookup(receiver, name, word_node),
			});
		}
		if !(is_loaded_word && type_module.is_none())
			&& self.context.user_functions.get(name).is_some_and(|function| !function.params.is_empty())
		{
			return Some(word_call(word_node, receiver, arguments));
		}
		// `x.string`, `t.int`, `x.float()`, `2.double()`: the conversion `string(x)`, as written in call form; uncalled
		// only the common conversions (not `e.text`, a common field); a counting word stays a count (`xs.number()`)
		let is_type_word_call = is_called
			&& crate::analyzer::type_word_kind(name).is_some()
			&& counting_method(name, &self.context).is_none();
		let is_conversion = CONVERSION_METHODS.contains(&name.as_str()) || is_type_word_call;
		if is_conversion && arguments.is_empty() && !self.instances.is_declared_field(name) {
			return Some(word_call(word_node, receiver, vec![]));
		}
		let is_known = self.is_known_method(receiver, name);
		let is_object = literal.is_some() || self.is_object(receiver, name) || self.may_hold_object(receiver);
		if is_object && !is_known && !has_arguments {
			return Some(field_lookup(receiver, name, word_node));
		}
		// `x.round`, `x.sqrt`: the number word applied to x, as `x.round()` and `sqrt x` are (x.word is word(x))
		if !has_arguments {
			if let Some(operation) = prefix_operator_method(receiver, name) {
				return Some(operation);
			}
			if crate::wasm_emitter::ROUNDING_FUNCTIONS.contains(&name.as_str()) {
				return Some(word_call(word_node, receiver, vec![]));
			}
		}
		// `x.type`, `x.type()`: `type(x)`, once no field of that name can be meant
		if name == crate::type_tests::TYPE_WORD && !has_arguments {
			return Some(word_call(word_node, receiver, vec![]));
		}
		let is_value = matches!(
			receiver.drop_meta(),
			Node::Symbol(_) | Node::Text(_) | Node::Char(_) | Node::Number(_) | Node::List(_, Bracket::Square, _)
		);
		let call = key(receiver.clone(), Op::Dot, method.clone());
		(!is_known && is_value).then(|| self.context.undefined_function_diagnostic(&call, name).into_error())
	}

	/// `x.square` and `x.add(y)` call the user function with the receiver as first argument; `l.start` of an object
	/// with a field start reads the field, even beside a function start (`start(l) := l.start`)
	fn reads_declared_field(&self, receiver: &Node, name: &str) -> bool {
		(self.instances.is_declared_field(name)
			&& (self.is_parameter(receiver) || self.instances.shape(receiver).is_some()))
			|| self.reads_own_parameter_field(receiver, name)
	}

	/// The type module (lib/list.warp, text.warp) of a loaded word of one parameter, which `x.word` calls
	fn type_module_of(&self, name: &str) -> Option<&'static str> {
		let takes_one = self.context.user_functions.get(name).is_some_and(|function| function.params.len() == 1);
		type_module_defining(name).filter(|_| takes_one)
	}

	/// A word a value answers other than by a field: a count, a list change, a user function, or `f.effects` of a
	/// function (not of a variable), the query effects.rs answers
	fn is_known_method(&self, receiver: &Node, name: &str) -> bool {
		let is_effects_query = name == crate::analyzer::effects::EFFECTS_WORD
			&& receiver.symbol_name().is_some_and(|function| !self.plain.contains(function));
		counting_method(name, &self.context).is_some()
			|| is_list_mutating_method(name)
			|| self.context.user_functions.contains_key(name)
			|| is_effects_query
	}

	/// A receiver known to be an object: a field lookup, a parameter, a call's result, or one of a declared field name
	fn is_object(&self, receiver: &Node, name: &str) -> bool {
		let is_call_result =
			matches!(receiver.drop_meta(), Node::Symbol(variable) if self.call_results.contains(variable));
		is_field_lookup(receiver)
			|| self.is_parameter(receiver)
			|| is_call_result
			|| self.instances.is_declared_field(name)
	}

	fn may_hold_object(&self, receiver: &Node) -> bool {
		// a variable of unknown value (`o` of `for o in people.filter(…)`), or an element of one (`xs[1]`), may hold an
		// object: its field is looked up at run time
		// a parenthesized receiver is what it holds: `(x#2).name` is `x#2.name` (card name-quoted); `f()` stays a call
		let inner = match receiver.drop_meta() {
			Node::List(items, Bracket::Round, _)
				if items.len() == 1 && !matches!(items[0].drop_meta(), Node::Symbol(_)) =>
			{
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
			Node::List(items, Bracket::Round, Separator::None)
				if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) =>
			{
				true
			}
			// a parenthesized object `({a:1}).a`, or quoted code `(data n*1).op` (node_api.rs)
			Node::List(_, Bracket::Curly, _) => true,
			_ if crate::blocks::quoted_data(variable).is_some() => true,
			// an element of a computed list: `users.filter(…)#1.name`
			_ => matches!(inner, Node::Key(_, Op::Hash, _)) && !is_plain_literal(variable),
		}
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
				let name = if matches!(head.drop_meta(), Node::Symbol(written) if written == word) {
					head.clone()
				} else {
					// the written spelling's position: `warn "x"` errs where warn is
					symbol(word).with_meta_of(head)
				};
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
		let heads_with_word = matches!(rest[0].drop_meta(), Node::Symbol(name) if self.library_word(name).is_some());
		let phrase = heads_with_word.then(|| self.word_call(rest, &Bracket::None, &Separator::Space)).flatten();
		let phrase = phrase.unwrap_or_else(|| Node::List(rest.to_vec(), Bracket::None, Separator::Space));
		[kept.to_vec(), vec![phrase]].concat()
	}

	/// The template of an expanded word with the receiver (held once in a temporary) and the further arguments in place
	fn expanded(&self, template: &str, arguments: Vec<Node>) -> Node {
		let program = named_apart(
			parse(&format!("({TEMPORARY}={RECEIVER_PLACEHOLDER}; {template})")),
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
fn is_comparison(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, op, _) if op.is_comparison() || op.is_logical())
}

/// The leftmost operand of a comparison chain, replaced: `"a" == b and c` → `f("a") == b and c`
fn with_first_comparand(node: Node, replace: &dyn Fn(Node) -> Node) -> Node {
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

/// An operand as written in a warning: a call `f 4` / `f(4)` as f(4), anything else as normalize writes it
fn call_text(node: &Node) -> String {
	match node.drop_meta() {
		Node::List(items, Bracket::None | Bracket::Round, _)
			if items.len() > 1 && matches!(items[0].drop_meta(), Node::Symbol(_)) =>
		{
			format!(
				"{}({})",
				items[0].drop_meta().name(),
				items[1..].iter().map(call_text).collect::<Vec<_>>().join(", ")
			)
		}
		_ => crate::normalize::operand_text(node),
	}
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
