//! Type tests: `x is int`, `x is a number`, `[1 2] is list of int`, `[1 2] is ints`, `x is pair`, `p is friend` (a declared
//! type) lower to `is_type(x, "spec")`, which the emitter answers in any position from the static type name of `x` (the same
//! as `type(x)`) when it is known, else from the value's kind at run time (`runtime_kind_mask`, node_kind_in) or, for a
//! declared type, its instance type (instance_of). `type of x` is `type(x)` (of_lookups.rs).
//! A type word on the right of `is` switches from equality to a type test; `x is y` with a variable stays equality.
//! `x == int` is no type test (user decision #30): a value never equals a type, so it is false and hints `x is int`.

use super::memoization::{definition_parts, Rebuild};
use super::nodes::{call, children_rewritten, is_call_head, key, or_operands, parameter_name};
use super::switch::is_pattern;
use crate::analyzer::{plural_element_type, type_word_kind};
use crate::library_words::collect_assigned_names;
use crate::node::{parens, text, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{
	ERROR_WORD, FLOAT_WORD, IS_TYPE, LIST_WORD, MAP_WORD, NUMBER_WORD, OF_WORD, OPERATION_WORD, PAIR_WORD, REAL_WORD,
	SYMBOL_WORD, TEXT_WORD,
};
use std::collections::HashSet;

/// `node_kind_in(node, mask)`: 1 when the run-time kind of node is in the bit mask of kinds (wasm_emitter list_ops.rs)
pub const NODE_KIND_IN: &str = "node_kind_in";
/// `node_type_name(node)`: `type(x)` of a value of unknown static type, its run-time type as a symbol (list_ops.rs)
pub const NODE_TYPE_NAME: &str = "node_type_name";
/// `node_is_map(node)`: 1 when the node is a map at run time, a curly list of pairs; quoted code `{print 1}` is none
pub const NODE_IS_MAP: &str = "node_is_map";
/// The type of ø, as type(ø) names it; unit, nil … are its aliases (canonical_spec_word)
pub const EMPTY_TYPE: &str = "empty";
const ARTICLES: [&str; 2] = ["a", "an"];
const SHADOWED_TYPE_WORD_TOPIC: &str = "shadowed-type-word";
/// `String(x)`, `x as String`: the class spelling of text, a cast word only, so `String` stays free as a type value
const TEXT_CLASS_WORD: &str = "String";
/// The collection types: `x is list`, `x is map`, and of their element type: `x is list of int`, `x is map of text`
pub(crate) const COLLECTION_WORDS: [&str; 2] = [LIST_WORD, MAP_WORD];
pub const TYPE_WORD: &str = "type";
/// The type of a built-in word's function value, a function the language provides (decision dir-lists)
pub const BUILTIN_TYPE: &str = "builtin";
/// A built-in type: its name, the other words naming it, the types it covers (`is` tests the subtype, `==` the type
/// itself), the kind a declared value of it is held as (`x: real = 1.5` an exact Int, `x: number` a Float; none for a
/// type no declaration converts to) and the bits of its run-time kinds (node_kind_in, none for a type only the static
/// type answers); a bool is an Int marked bool and has a bit of its own
struct BuiltinType {
	name: &'static str,
	aliases: &'static [&'static str],
	covers: &'static [&'static str],
	held_as: Option<K>,
	kind_bits: &'static [i64],
}

const fn builtin(
	name: &'static str,
	aliases: &'static [&'static str],
	covers: &'static [&'static str],
	held_as: Option<K>,
	kind_bits: &'static [i64],
) -> BuiltinType {
	BuiltinType { name, aliases, covers, held_as, kind_bits }
}

use crate::type_kinds::Kind as K;
/// The built-in type words, the one list of them (card cleanup-closed-lists): each is a type value where it stands bare
/// (`t = int`, `type(π) == real`). `number` covers every number, `real` the exact numbers and π, `rational` the whole
/// numbers too, `text` a one-character string (a codepoint); `list` and ø both hold the empty list, as `count` takes
/// it; a ± value is a number. The fixed widths (`int8`, `uint16` …) are src/numbers/fixed_width.rs
const BUILTIN_TYPES: [BuiltinType; 26] = [
	builtin("int", &["integer", "long", "i64", "i32"], &[], Some(K::Int), &[K::Int as i64]),
	builtin("rational", &[], &["int"], Some(K::Int), &[K::Int as i64, K::Float as i64]),
	builtin(REAL_WORD, &["exact"], &["rational"], Some(K::Empty), &[K::Int as i64, K::Float as i64]),
	builtin(
		FLOAT_WORD,
		&["double", "f64", "f32", "float32", "float64", "fast"],
		&[],
		Some(K::Float),
		&[K::Float as i64],
	),
	builtin("number", &[], &["real", "float"], Some(K::Float), &[K::Int as i64, K::Float as i64, K::Uncertain as i64]),
	builtin(TEXT_WORD, &["str", "string"], &["codepoint"], Some(K::Text), &[K::Text as i64, K::Codepoint as i64]),
	builtin("codepoint", &["char", "character"], &[], Some(K::Codepoint), &[K::Codepoint as i64]),
	builtin(crate::analyzer::BOOL_TYPE, &["boolean"], &[], Some(K::Int), &[crate::type_kinds::BOOL_MASK_BIT]),
	builtin("function", &["closure"], &[BUILTIN_TYPE], Some(K::Function), &[]),
	builtin(BUILTIN_TYPE, &[], &[], Some(K::Function), &[]),
	// a function or a block (card callable): what a `f: callable` parameter takes (lowering/control/blocks.rs)
	builtin(
		crate::blocks::CALLABLE_WORD,
		&["invocable", "runnable"],
		&["function", crate::words::CODE_WORD],
		None,
		&[],
	),
	builtin(SYMBOL_WORD, &[], &[], Some(K::Symbol), &[K::Symbol as i64]),
	builtin("key", &[PAIR_WORD], &[OPERATION_WORD], None, &[K::Key as i64, crate::type_kinds::OPERATION_MASK_BIT]),
	builtin(OPERATION_WORD, &["expression"], &[], None, &[crate::type_kinds::OPERATION_MASK_BIT]),
	builtin(crate::words::CODE_WORD, &[], &[], None, &[crate::type_kinds::CODE_MASK_BIT]),
	builtin(EMPTY_TYPE, &["unit", "nil", "ø", "none", "null", "void"], &[], None, &[K::Empty as i64]),
	builtin(ERROR_WORD, &[], &[], None, &[K::Error as i64]),
	builtin(LIST_WORD, &[], &[], None, &[K::List as i64, K::Block as i64, K::Empty as i64]),
	builtin(MAP_WORD, &[], &[], None, &[]),
	builtin(crate::time::TIME_WORD, &[], &TIME_FORM_NAMES, None, &[K::Time as i64]),
	// the forms share Kind::Time: a value of unknown static type tests its form by its type name (node_type_name)
	builtin(TIME_FORM_NAMES[0], &[], &[], None, &[]),
	builtin(TIME_FORM_NAMES[1], &[], &[], None, &[]),
	builtin(TIME_FORM_NAMES[2], &[], &[], None, &[]),
	builtin(TIME_FORM_NAMES[3], &[], &[], None, &[]),
	builtin(TIME_FORM_NAMES[4], &[], &[], None, &[]),
	builtin(TIME_FORM_NAMES[5], &[], &[], None, &[]),
];
const TIME_FORM_NAMES: [&str; 6] = {
	let forms = crate::time::TIME_FORMS;
	[forms[0].name(), forms[1].name(), forms[2].name(), forms[3].name(), forms[4].name(), forms[5].name()]
};

fn builtin_type(word: &str) -> Option<&'static BuiltinType> {
	BUILTIN_TYPES.iter().find(|builtin| builtin.name == word || builtin.aliases.contains(&word))
}

/// The name of the built-in type a word or an alias names: `integer` → int
pub fn builtin_type_name(word: &str) -> Option<&'static str> {
	builtin_type(word).map(|builtin| builtin.name)
}

/// A word naming a built-in type or an alias of one (BUILTIN_TYPES): where it stands bare, it is that type as a
/// value, never an undefined variable
pub fn is_builtin_type_word(word: &str) -> bool {
	builtin_type(word).is_some()
}

/// Every word naming a built-in type, aliases too
pub fn builtin_type_words() -> impl Iterator<Item = &'static str> {
	BUILTIN_TYPES.iter().flat_map(|builtin| std::iter::once(builtin.name).chain(builtin.aliases.iter().copied()))
}

/// `int`, `i64`, `exact`, `real`, `f64` …: a word naming a type of numbers (one number is covers), `number` itself not
pub fn is_number_type_word(word: &str) -> bool {
	let name = canonical_spec_word(word);
	is_builtin_type_word(word) && name != NUMBER_WORD && type_matches(name, NUMBER_WORD)
}

/// `x as str`, `String(x)`: a word whose cast or call gives a value's text
pub fn is_text_type_word(word: &str) -> bool {
	word == TEXT_CLASS_WORD || canonical_spec_word(word) == TEXT_WORD
}

/// `x as double`, `x as f32`: a word naming the IEEE float type
pub fn is_float_type_word(word: &str) -> bool {
	canonical_spec_word(word) == FLOAT_WORD
}

/// `number`: a place declared so keeps the number it is given, exact or float, converting none (decisions
/// rational-float, exact-decimals; card number-keeps-exact)
pub fn holds_any_number(word: &str) -> bool {
	canonical_spec_word(word) == NUMBER_WORD
}

/// `real` or `exact`: a value declared so keeps an exact real (`√2`, `π`) as it is
pub fn is_real_type_word(word: &str) -> bool {
	canonical_spec_word(word) == REAL_WORD
}

/// Words that name the same type
pub(crate) fn canonical_spec_word(word: &str) -> &str {
	builtin_type(word).map_or(word, |builtin| builtin.name)
}

/// The kind a value declared of the built-in type `word` is held as (analyzer builtin_type_kind)
pub fn held_kind(word: &str) -> Option<K> {
	builtin_type(word)?.held_as
}

/// `rational`, `real`, `exact`: the exact numbers beyond the whole ones, the types covering int that cover no float
pub fn is_exact_fraction_type(word: &str) -> bool {
	let name = canonical_spec_word(word);
	name != "int" && type_matches("int", name) && !type_matches("float", name)
}

/// A decimal written for this type is the exact fraction it spells: the exact fractions and `real`, whose exact reals
/// also hold floats (decision exact-default: elsewhere a decimal is a float)
pub fn spells_decimals_exactly(word: &str) -> bool {
	is_exact_fraction_type(word) || canonical_spec_word(word) == "real"
}

/// Does a value of the static type name `actual` (`type(x)`) have the type `spec`: actual is spec or a type spec covers,
/// `list of number` every list of numbers
pub fn type_matches(actual: &str, spec: &str) -> bool {
	let spec = canonical_spec_word(spec);
	if let Some(conforms) = crate::traits::builtin_conforms(actual, spec) {
		return conforms;
	}
	for collection in COLLECTION_WORDS {
		let of = format!("{collection} of ");
		if let Some(element) = spec.strip_prefix(&of) {
			return actual.strip_prefix(&of).is_some_and(|actual_element| type_matches(actual_element, element));
		}
		if spec == collection {
			return actual == collection || actual.starts_with(&of);
		}
	}
	actual == spec
		|| builtin_type(spec).is_some_and(|builtin| builtin.covers.iter().any(|covered| type_matches(actual, covered)))
}

/// The built-in types a value of type `spec` may have: `t is number` of a type value t is t == one of them
pub fn builtin_types_matching(spec: &str) -> Vec<&'static str> {
	BUILTIN_TYPES.iter().map(|builtin| builtin.name).filter(|name| type_matches(name, spec)).collect()
}

/// The run-time kinds of a value of type `spec` as a mask of their bits (node_kind_in), for a value whose static type is
/// unknown (an item of a mixed list, a Node); None for a spec only the static type answers
pub fn runtime_kind_mask(spec: &str) -> Option<i64> {
	let spec = canonical_spec_word(spec);
	let spec = if spec.starts_with("list of ") { LIST_WORD } else { spec };
	let kind_bits = builtin_type(spec)?.kind_bits;
	(!kind_bits.is_empty()).then(|| kind_bits.iter().fold(0, |mask, bit| mask | 1 << bit))
}

/// The variables in scope (a name of one is no type in a test) and the program's declared types (`class friend`: `x is friend`)
#[derive(Default, Clone)]
pub(crate) struct Names {
	variables: HashSet<String>,
	types: HashSet<String>,
	/// The loop variables anywhere in the program (`for char in s`): such a name compared with == is the variable
	loop_variables: HashSet<String>,
	/// The program's functions that need arguments, a bare name of one is the function in a type test (`f is function`)
	functions_taking_arguments: HashSet<String>,
	/// Every name the program binds other than as a function: variables, parameters (of lambdas too), loop variables,
	/// fields; such a name anywhere is never taken for a function of that name in a type test
	bound_otherwise: HashSet<String>,
}

impl Names {
	pub(crate) fn contains(&self, name: &str) -> bool {
		self.variables.contains(name)
	}

	/// A variable or a loop variable of the program
	pub(crate) fn is_variable(&self, name: &str) -> bool {
		self.contains(name) || self.loop_variables.contains(name)
	}

	/// A bare name of a function that needs arguments, the program's or the library's (`floor`), and no variable: it
	/// can only mean the function itself (card bare-fn-type)
	fn is_function_needing_arguments(&self, name: &str) -> bool {
		let library_word_taking_arguments = || {
			!self.is_variable(name)
				&& !self.types.contains(name)
				&& !is_builtin_type_word(name)
				&& super::library_signatures::library_signature(name).is_some_and(|(params, _)| !params.is_empty())
		};
		!self.bound_otherwise.contains(name)
			&& !self.loop_variables.contains(name)
			&& (self.functions_taking_arguments.contains(name) || library_word_taking_arguments())
	}

	/// In the body of a function: also its parameters and the names it assigns, which no other function sees
	fn within(&self, head: &Node, body: &Node) -> Names {
		let mut bound = HashSet::new();
		if let Node::List(items, _, _) = head.drop_meta() {
			bound.extend(items.iter().skip(1).filter_map(parameter_name));
		}
		collect_assigned_names(body, &mut bound);
		let mut inner = self.clone();
		inner.variables.extend(bound.iter().cloned());
		// a parameter or local named like a function, `f(name) := name is text` of a method name(), is the value
		inner.bound_otherwise.extend(bound);
		inner
	}
}

/// `f(x) := body`, not `x := value`: its head and body
pub(crate) fn function_definition(node: &Node) -> Option<(Node, Node, Rebuild)> {
	definition_parts(node).filter(|(head, _, _)| is_call_head(head))
}

/// The names assigned outside the bodies of functions, which every function sees
pub(crate) fn collect_outer_assigned_names(node: &Node, names: &mut HashSet<String>) {
	if function_definition(node).is_some() {
		return;
	}
	if let Node::Key(target, Op::Assign | Op::Define, _) = node.drop_meta() {
		names.extend(target.symbol_name().map(String::from));
	}
	for part in node.drop_meta().parts() {
		collect_outer_assigned_names(part, names);
	}
}

pub fn lower(node: Node) -> Node {
	let names = program_names(&node);
	expand(node, &names)
}

/// The program's variables, functions and declared types
pub(crate) fn program_names(node: &Node) -> Names {
	let context = crate::analyzer::function_definitions(node);
	// a function named like a type is a class's printing method (`text() := …`, card is-text-method): no shadow, a
	// program's own `text(x) := …` is refused as "text is a type"
	let mut variables: HashSet<String> =
		context.user_functions.keys().filter(|name| !is_builtin_type_word(name)).cloned().collect();
	collect_outer_assigned_names(node, &mut variables);
	let mut bound_otherwise = crate::program_names::Bindings::of(node).bound();
	bound_otherwise.retain(|name| !context.user_functions.contains_key(name));
	let functions_taking_arguments = context
		.user_functions
		.values()
		.filter(|function| !function.params.is_empty())
		.map(|function| function.name.clone())
		.collect();
	let mut registry = crate::type_kinds::TypeRegistry::new();
	crate::analyzer::collect_all_types(&mut registry, node);
	let types = registry.types().iter().map(|definition| definition.name.clone()).collect();
	let loop_variables = super::for_loop::loop_variables(node).into_iter().collect();
	Names { variables, types, loop_variables, functions_taking_arguments, bound_otherwise }
}

/// `is_type(subject, spec)` when the word names a type (`int`, `ints`, `Circle`) and no variable
pub(crate) fn type_test(subject: &Node, word: &Node, names: &Names) -> Option<Node> {
	let word = word.symbol_name()?;
	type_spec(&[word], names).map(|spec| is_type_call(subject.clone(), spec, names))
}

/// `x is (int or text)`: a type union, x is one of the types (`or` is also the type union, user); a value among them
/// keeps the comparison
fn union_test(subject: &Node, union: &Node, names: &Names) -> Option<Node> {
	let types = union_operands(union);
	if types.len() < 2 || is_equality_operand(union) {
		return None;
	}
	let tests: Option<Vec<Node>> = types.iter().map(|word| type_test(subject, word, names)).collect();
	tests?.into_iter().reduce(|all, test| key(all, Op::Or, test))
}

/// The operands of `(a or b or c)`
fn union_operands(union: &Node) -> Vec<Node> {
	match union.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => or_operands(&items[0]),
		_ => or_operands(union),
	}
}

/// `int or text` of only type words, compared with `==`, stays a value (user decision #30, as `x == int`)
fn is_type_union(union: &Node) -> bool {
	let types = union_operands(union);
	types.len() > 1
		&& types.iter().all(|word| matches!(word.drop_meta(), Node::Symbol(name) if is_builtin_type_word(name)))
}

/// The spec of a type phrase (`int`, `a number`, `ints`, `list of int`, `list of list of int`), `None` when the words are no type
fn type_spec(words: &[&str], shadowed: &Names) -> Option<String> {
	// `x is a key` tests the type even where a variable is named key
	let (words, has_article) = match words {
		[article, rest @ ..] if ARTICLES.contains(article) && !rest.is_empty() => (rest, true),
		all => (all, false),
	};
	let (first, rest) = words.split_first()?;
	if !has_article && shadowed.contains(first) {
		return None;
	}
	let phrase = words.join(" ");
	match (*first, rest) {
		(word, []) if is_builtin_type_word(word) => Some(canonical_spec_word(word).to_string()),
		(_, [_, ..]) if is_builtin_type_word(&phrase) => Some(canonical_spec_word(&phrase).to_string()),
		(collection, [of, element @ ..]) if COLLECTION_WORDS.contains(&collection) && *of == OF_WORD => {
			Some(format!("{collection} of {}", type_spec(element, shadowed)?))
		}
		(word, []) => match plural_element_type(word) {
			Some(element) => Some(format!("{LIST_WORD} of {}", canonical_spec_word(element))),
			None if crate::traits::is_builtin_trait(word) || shadowed.types.contains(word) => Some(word.to_string()),
			None => type_word_kind(word).map(|_| canonical_spec_word(word).to_string()),
		},
		_ => None,
	}
}

fn symbol_words(nodes: &[Node]) -> Option<Vec<&str>> {
	nodes.iter().map(Node::symbol_name).collect()
}

/// The words of `x is int` and of `x is a number`, whose `a number` parses as a call (application binds tighter than `is`)
fn phrase_words(node: &Node) -> Option<Vec<&str>> {
	match node.drop_meta() {
		Node::List(items, Bracket::None, Separator::Space) => {
			items.iter().map(phrase_words).collect::<Option<Vec<_>>>().map(|words| words.concat())
		}
		_ => symbol_words(std::slice::from_ref(node)),
	}
}

/// Meta key marking a type word compared with `==` (not `is`): only `is` tests types (user decision #30)
const EQUALITY_OPERAND: &str = "equality operand";

/// A side of `x == word` or `word != x` as the parser marks it when the word names a type; lowering names it by its
/// canonical name as type(x) gives it (`type(s) == string` is `type(s) == text`) unless it is a variable
/// (canonical_operand). The words of ø (empty, nil, none …) parse as the value ø, the one value of the empty type:
/// compared with a type value (`type(x) == empty`) ø names that type
pub fn equality_operand(subject: &Node, word: Node) -> Node {
	match word.drop_meta() {
		Node::Symbol(name) if type_spec(&[name.as_str()], &Names::default()).is_some() => marked_operand(name),
		Node::Empty if typed_argument(subject).is_some() => marked_operand(EMPTY_TYPE),
		_ if is_type_union(&word) => marked(word),
		_ => word,
	}
}

fn marked_operand(word: &str) -> Node {
	marked(Node::Symbol(word.to_string()))
}

fn marked(operand: Node) -> Node {
	Node::Meta { node: Box::new(operand), data: Box::new(Node::key(EQUALITY_OPERAND, Node::True)) }
}

/// `left == right` or `left != right`, a type word on either side by its canonical name
fn comparison(left: Node, op: Op, right: Node, shadowed: &Names) -> Node {
	hint_type_compared_with_text(&left, &op, &right);
	hint_type_compared_with_text(&right, &op, &left);
	if let Some(answer) = function_compared_with_type(&left, &op, &right, shadowed) {
		return answer;
	}
	key(canonical_operand(expand(left, shadowed), shadowed), op, canonical_operand(expand(right, shadowed), shadowed))
}

/// `type(x) == "int"` is no: a type never equals a text, though it prints as one (decision type-equal), so the
/// comparison hints the type word (card type-text-compare)
fn hint_type_compared_with_text(type_value: &Node, op: &Op, other: &Node) {
	let (Some(argument), Node::Text(word)) = (compared_type_argument(type_value), other.drop_meta()) else {
		return;
	};
	if type_spec(&[word.as_str()], &Names::default()).is_none() {
		return;
	}
	let typed = format!("{TYPE_WORD}({})", crate::normalize::operand_text(&argument));
	let joined = typed_argument(type_value).is_none();
	let written = if joined { crate::normalize::operand_text(type_value) } else { typed.clone() };
	crate::normalize::set_position_of(other);
	crate::normalize::hint(
		&format!("{written} {op} \"{word}\""),
		&format!("{typed} {op} {word}"),
		"a type never equals a text; compare with the type word",
	);
}

/// The x of `type(x)`, also joined to a text: `"" + type(x)` is the text `class int`, never "int" (decision type-text)
fn compared_type_argument(node: &Node) -> Option<Node> {
	match node.drop_meta() {
		Node::Key(left, Op::Add, right) => compared_type_argument(left).or_else(|| compared_type_argument(right)),
		_ => typed_argument(node),
	}
}

/// A marked side of == or != by the canonical name of its type; a variable of that name stays the variable
fn canonical_operand(operand: Node, shadowed: &Names) -> Node {
	match operand.drop_meta() {
		Node::Symbol(word)
			if is_equality_operand(&operand) && (shadowed.contains(word) || shadowed.loop_variables.contains(word)) =>
		{
			Node::Symbol(word.clone())
		}
		Node::Symbol(word) if is_equality_operand(&operand) => {
			marked_operand(&type_spec(&[word.as_str()], &Names::default()).unwrap_or_else(|| word.clone()))
		}
		_ => operand,
	}
}

/// Meta key on the name compared in `x is v` (set by the parser): v as written, before it is lowered (`100 times [0]`), for the
/// error that teaches `x be v` when x is defined nowhere (P61, wasm_emitter emit_undefined_comparison)
pub const COMPARED_WITH: &str = "compared with";

pub fn with_compared_text(subject: Node, written: &str) -> Node {
	match subject.drop_meta() {
		// a literal too: `30 is Age` is a test type_aliases.rs resolves the alias of (card alias-is-literal)
		Node::Symbol(_) | Node::Number(_) | Node::Text(_) | Node::Char(_) => {
			Node::Meta { node: Box::new(subject), data: Box::new(Node::key(COMPARED_WITH, text(written))) }
		}
		_ => subject,
	}
}

/// The written value a compared name carries (with_compared_text)
pub fn compared_text(subject: &Node) -> Option<String> {
	subject.marked(COMPARED_WITH).map(|(_, value)| value.drop_meta().name())
}

pub fn is_equality_operand(node: &Node) -> bool {
	matches!(node, Node::Meta { data, .. } if matches!(data.as_ref(), Node::Key(key, _, _) if key.name() == EQUALITY_OPERAND))
}

/// The right side of `x is int` or `x is a number` when it begins a type test, which no runtime's `==` may take over
/// (foreign_modules.rs: `math.floor(2.5) is int` tests the result's kind, not `operator.eq(…, int)`)
pub(crate) fn begins_type_test(right: &Node) -> bool {
	!is_equality_operand(right)
		&& phrase_words(right).is_some_and(|words| {
			words.first().is_some_and(|first| ARTICLES.contains(first))
				|| type_spec(&words, &Names::default()).is_some()
		})
}

/// `is_type(value, spec)` of a declared parameter type (`text`, `ints`, `list of int`, `map`); None for no type word
pub(crate) fn declared_type_test(subject: Node, type_name: &str) -> Option<Node> {
	let words: Vec<&str> = type_name.split(' ').collect();
	let names = Names::default();
	type_spec(&words, &names).map(|spec| is_type_call(subject, spec, &names))
}

/// `x is int`; `type(x) is int` tests x, as the type of x is what it names (card type-int)
fn is_type_call(subject: Node, spec: String, names: &Names) -> Node {
	let tested = typed_argument(&subject).unwrap_or(subject);
	call(IS_TYPE, vec![function_itself(tested, names), Node::Text(spec)])
}

/// `floor` of `floor is function`: a bare name of a function that needs arguments, tested against a type, is the
/// function itself (card bare-fn-type)
fn function_itself(operand: Node, names: &Names) -> Node {
	match operand.drop_meta() {
		Node::Symbol(name) if names.is_function_needing_arguments(name) => {
			crate::closures::function_reference(name.clone())
		}
		_ => operand,
	}
}

/// `floor == function`: a function never equals a type (user decision #30), so no (`!=`: yes), hinting `floor is
/// function`
fn function_compared_with_type(left: &Node, op: &Op, right: &Node, names: &Names) -> Option<Node> {
	let (function, type_word) = match (is_type_operand(left, names), is_type_operand(right, names)) {
		(false, true) => (left, right),
		(true, false) => (right, left),
		_ => return None,
	};
	let name = function.drop_meta().symbol_name().filter(|name| names.is_function_needing_arguments(name))?;
	let type_word = type_word.drop_meta().name();
	let test = if *op == Op::Ne { "is not" } else { "is" };
	crate::normalize::set_position_of(function);
	crate::normalize::hint(
		&format!("{name} {op} {type_word}"),
		&format!("{name} {test} {type_word}"),
		"only `is` tests a type; a function never equals a type",
	);
	Some(if *op == Op::Ne { Node::True } else { Node::False })
}

/// A type word on a side of == (`int`, `function`)
fn is_type_operand(operand: &Node, names: &Names) -> bool {
	is_equality_operand(operand)
		&& matches!(operand.drop_meta(), Node::Symbol(word) if type_spec(&[word.as_str()], names).is_some())
}

/// x of `type(x)`
fn typed_argument(node: &Node) -> Option<Node> {
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 && items[0].is_symbol(TYPE_WORD) => Some(items[1].clone()),
		_ => None,
	}
}

fn expand(node: Node, shadowed: &Names) -> Node {
	if let Some((head, body, rebuild)) = function_definition(&node) {
		let inner = shadowed.within(&head, &body);
		return rebuild(head, expand(body, &inner));
	}
	match node {
		Node::List(items, Bracket::None | Bracket::Round, _) if names_a_type(&items, shadowed) => {
			Node::Symbol(TYPE_WORD.to_string())
		}
		Node::List(items, bracket, separator) => {
			if let Some(call) = spaced_type_test(&items, shadowed) {
				return expand(call, shadowed);
			}
			Node::List(items.into_iter().map(|item| expand(item, shadowed)).collect(), bracket, separator)
		}
		Node::Key(subject, Op::Eq, right) => {
			warn_shadowed_type_word(&subject, &right, shadowed);
			match phrase_words(&right).and_then(|words| type_spec(&words, shadowed)) {
				// `x == int` compares x with the type value int: equal only to that type (wasm_emitter emit_structural_equality)
				Some(spec) if !is_equality_operand(&right) => is_type_call(expand(*subject, shadowed), spec, shadowed),
				_ => match union_test(&subject, &right, shadowed).or_else(|| is_pattern(&subject, &right, shadowed)) {
					Some(test) => expand(test, shadowed),
					None => comparison(*subject, Op::Eq, *right, shadowed),
				},
			}
		}
		Node::Key(left, Op::Ne, right) => comparison(*left, Op::Ne, *right, shadowed),
		Node::Key(..) if let Some((subject, types)) = union_after_is(&node, shadowed) => {
			let union = types.into_iter().reduce(|all, part| key(all, Op::Or, part)).expect("two types");
			expand(key(subject, Op::Eq, parens(vec![union])), shadowed)
		}
		other => children_rewritten(other, |child| expand(child, shadowed)),
	}
}

/// `n is key` with a variable named key compares with its value: warn, never silently (card local-named-like-2)
fn warn_shadowed_type_word(subject: &Node, right: &Node, shadowed: &Names) {
	let Node::Symbol(word) = right.drop_meta() else {
		return;
	};
	if is_equality_operand(right)
		|| !shadowed.contains(word)
		|| type_spec(&[word.as_str()], &Names::default()).is_none()
	{
		return;
	}
	let tested = crate::normalize::operand_text(subject);
	let article = if word.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
	let readings = vec![
		crate::errors::diagnostic::reading("the variable's value", &format!("{tested} == {word}")),
		crate::errors::diagnostic::reading("the type", &format!("{tested} is {article} {word}")),
	];
	let question = crate::errors::diagnostic::Ask::new(
		SHADOWED_TYPE_WORD_TOPIC,
		format!("`{word}` is a variable here, so `{tested} is {word}` compares with its value, not the type {word}: rename the variable, or write `{tested} is {article} {word}` for the type"),
		readings,
		crate::errors::diagnostic::Fallback::Warning,
	)
	.written(&format!("{tested} is {word}"))
	.at_node(subject);
	// a warning only: the default reading (the variable) is what the program gets
	let _ = crate::errors::diagnostic::ask(&question);
}

/// `v is int or text` parses as `(v is int) or text`: after `is` the union is the type, `or` spells a union in every
/// type position (decision union-return, card int-text); a non-type operand (`v is int or ok`) keeps the parse
fn union_after_is(node: &Node, names: &Names) -> Option<(Node, Vec<Node>)> {
	let Node::Key(left, Op::Or, rest) = node.drop_meta() else {
		return None;
	};
	let (subject, mut types) = match left.drop_meta() {
		Node::Key(subject, Op::Eq, first) if !is_equality_operand(first) => {
			(subject.as_ref().clone(), vec![first.as_ref().clone()])
		}
		chain => union_after_is(chain, names)?,
	};
	types.extend(or_operands(rest));
	types.iter().all(|word| type_test(&subject, word, names).is_some()).then_some((subject, types))
}

/// `type(int)`, `type(Circle)`: the type of a type is type, unless a variable takes the name
fn names_a_type(items: &[Node], names: &Names) -> bool {
	let [word, subject] = items else { return false };
	let Some(name) = subject.symbol_name() else { return false };
	word.is_symbol(TYPE_WORD)
		&& !names.is_variable(TYPE_WORD)
		&& !names.is_variable(name)
		&& (is_builtin_type_word(name) || names.types.contains(name))
}

/// `x is a number` is the items `x is a` and `number`: the words after the comparison continue the type phrase
fn spaced_type_test(items: &[Node], shadowed: &Names) -> Option<Node> {
	let [first, tail @ ..] = items else { return None };
	let Node::Key(subject, Op::Eq, right) = first.drop_meta() else {
		return None;
	};
	if is_equality_operand(right) {
		return None;
	}
	let word = right.symbol_name()?;
	let mut words = vec![word];
	words.extend(symbol_words(tail)?);
	Some(is_type_call(subject.as_ref().clone(), type_spec(&words, shadowed)?, shadowed))
}
