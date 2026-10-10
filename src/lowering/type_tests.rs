//! Type tests: `x is int`, `x is a number`, `[1 2] is list of int`, `[1 2] is ints`, `x is pair`, `p is friend` (a declared
//! type) lower to `is_type(x, "spec")`, which the emitter answers in any position from the static type name of `x` (the same
//! as `type(x)`) when it is known, else from the value's kind at run time (`runtime_kind_mask`, node_kind_in) or, for a
//! declared type, its instance type (instance_of). `type of x` is `type(x)`.
//! A type word on the right of `is` switches from equality to a type test; `x is y` with a variable stays equality.
//! `x == int` is no type test (user decision #30): a value never equals a type, so it is false and hints `x is int`.

use super::memoization::{definition_parts, Rebuild};
use super::words::OF_WORD;
use super::nodes::{call, is_call_head, key, parameter_name};
use crate::analyzer::{plural_element_type, type_word_kind};
use crate::library_words::collect_assigned_names;
use crate::node::{text, Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;

pub const IS_TYPE: &str = "is_type";
/// `node_kind_in(node, mask)`: 1 when the run-time kind of node is in the bit mask of kinds (wasm_emitter list_ops.rs)
pub const NODE_KIND_IN: &str = "node_kind_in";
/// `node_type_name(node)`: `type(x)` of a value of unknown static type, its run-time type as a symbol (list_ops.rs)
pub const NODE_TYPE_NAME: &str = "node_type_name";
/// `x is error`: any value may turn out an Error at run time, so this test always reads the kind (card catch-message)
pub const ERROR_TYPE: &str = "error";
/// The type of ø, as type(ø) names it; unit, nil … are its aliases (canonical_spec_word)
pub const EMPTY_TYPE: &str = "empty";
const ARTICLES: [&str; 2] = ["a", "an"];
const LIST_WORD: &str = "list";
/// `x is pair`: a `key: value` pair
const PAIR_WORD: &str = "pair";
pub const TYPE_WORD: &str = "type";
pub const SYMBOL_TYPE: &str = "symbol";
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

const fn builtin(name: &'static str, aliases: &'static [&'static str], covers: &'static [&'static str], held_as: Option<K>, kind_bits: &'static [i64]) -> BuiltinType {
	BuiltinType { name, aliases, covers, held_as, kind_bits }
}

use crate::type_kinds::Kind as K;
/// The built-in type words, the one list of them (card cleanup-closed-lists): each is a type value where it stands bare
/// (`t = int`, `type(π) == real`). `number` covers every number, `real` the exact numbers and π, `rational` the whole
/// numbers too, `text` a one-character string (a codepoint); `list` and ø both hold the empty list, as `count` takes
/// it; a ± value is a number. The fixed widths (`int8`, `uint16` …) are src/fixed_width.rs
const BUILTIN_TYPES: [BuiltinType; 14] = [
	builtin("int", &["integer", "long", "i64", "i32"], &[], Some(K::Int), &[K::Int as i64]),
	builtin("rational", &["exact"], &["int"], Some(K::Int), &[K::Int as i64, K::Float as i64]),
	builtin("real", &[], &["rational"], Some(K::Int), &[K::Int as i64, K::Float as i64]),
	builtin("float", &["double", "f64", "f32", "float32", "float64", "fast"], &[], Some(K::Float), &[K::Float as i64]),
	builtin("number", &["num"], &["real", "float"], Some(K::Float), &[K::Int as i64, K::Float as i64, K::Uncertain as i64]),
	builtin("text", &["str", "string"], &["codepoint"], Some(K::Text), &[K::Text as i64, K::Codepoint as i64]),
	builtin("codepoint", &["char", "character"], &[], Some(K::Codepoint), &[K::Codepoint as i64]),
	builtin(crate::analyzer::BOOL_TYPE, &["boolean"], &[], Some(K::Int), &[crate::type_kinds::BOOL_MASK_BIT]),
	builtin("function", &["closure"], &[], Some(K::Function), &[]),
	builtin(SYMBOL_TYPE, &[], &[], None, &[K::Symbol as i64]),
	builtin("key", &[PAIR_WORD], &[], None, &[K::Key as i64]),
	builtin(EMPTY_TYPE, &["unit", "nil", "ø", "none", "null", "void"], &[], None, &[K::Empty as i64]),
	builtin(ERROR_TYPE, &[], &[], None, &[K::Error as i64]),
	builtin(LIST_WORD, &[], &[], None, &[K::List as i64, K::Block as i64, K::Empty as i64]),
];

fn builtin_type(word: &str) -> Option<&'static BuiltinType> {
	BUILTIN_TYPES.iter().find(|builtin| builtin.name == word || builtin.aliases.contains(&word))
}

/// A word naming a built-in type: where it stands bare, it is that type as a value, never an undefined variable
pub fn is_type_word(word: &str) -> bool {
	builtin_type(word).is_some()
}

/// Every word naming a built-in type, aliases too
pub fn builtin_type_words() -> impl Iterator<Item = &'static str> {
	BUILTIN_TYPES.iter().flat_map(|builtin| std::iter::once(builtin.name).chain(builtin.aliases.iter().copied()))
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

/// Does a value of the static type name `actual` (`type(x)`) have the type `spec`: actual is spec or a type spec covers,
/// `list of number` every list of numbers
pub fn type_matches(actual: &str, spec: &str) -> bool {
	let spec = canonical_spec_word(spec);
	if let Some(conforms) = crate::traits::builtin_conforms(actual, spec) {
		return conforms;
	}
	if let Some(element) = spec.strip_prefix("list of ") {
		return actual.strip_prefix("list of ").is_some_and(|actual_element| type_matches(actual_element, element));
	}
	if spec == LIST_WORD && actual.starts_with("list of ") {
		return true;
	}
	actual == spec || builtin_type(spec).is_some_and(|builtin| builtin.covers.iter().any(|covered| type_matches(actual, covered)))
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
struct Names {
	variables: HashSet<String>,
	types: HashSet<String>,
}

impl Names {
	fn contains(&self, name: &str) -> bool {
		self.variables.contains(name)
	}

	/// In the body of a function: also its parameters and the names it assigns, which no other function sees
	fn within(&self, head: &Node, body: &Node) -> Names {
		let mut inner = self.clone();
		if let Node::List(items, _, _) = head.drop_meta() {
			inner.variables.extend(items.iter().skip(1).filter_map(parameter_name));
		}
		collect_assigned_names(body, &mut inner.variables);
		inner
	}
}

/// `f(x) := body`, not `x := value`: its head and body
fn function_definition(node: &Node) -> Option<(Node, Node, Rebuild)> {
	definition_parts(node).filter(|(head, _, _)| is_call_head(head))
}

/// The names assigned outside the bodies of functions, which every function sees
fn collect_outer_assigned_names(node: &Node, names: &mut HashSet<String>) {
	if function_definition(node).is_some() {
		return;
	}
	match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, value) => {
			if let Node::Symbol(name) = target.drop_meta() {
				names.insert(name.clone());
			}
			collect_outer_assigned_names(value, names);
		}
		Node::Key(left, _, right) => {
			collect_outer_assigned_names(left, names);
			collect_outer_assigned_names(right, names);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| collect_outer_assigned_names(item, names)),
		_ => {}
	}
}

pub fn lower(node: Node) -> Node {
	let context = crate::analyzer::function_context(&node);
	let mut variables: HashSet<String> = context.user_functions.keys().cloned().collect();
	collect_outer_assigned_names(&node, &mut variables);
	let mut registry = crate::type_kinds::TypeRegistry::new();
	crate::analyzer::collect_all_types(&mut registry, &node);
	let types = registry.types().iter().map(|definition| definition.name.clone()).collect();
	expand(node, &Names { variables, types })
}

/// The spec of a type phrase (`int`, `a number`, `ints`, `list of int`, `list of list of int`), `None` when the words are no type
fn type_spec(words: &[&str], shadowed: &Names) -> Option<String> {
	let words = match words {
		[article, rest @ ..] if ARTICLES.contains(article) && !rest.is_empty() => rest,
		all => all,
	};
	let (first, rest) = words.split_first()?;
	if shadowed.contains(first) {
		return None;
	}
	match (*first, rest) {
		(word, []) if is_type_word(word) => Some(canonical_spec_word(word).to_string()),
		(LIST_WORD, [of, element @ ..]) if *of == OF_WORD => Some(format!("{LIST_WORD} of {}", type_spec(element, shadowed)?)),
		(word, []) => match plural_element_type(word) {
			Some(element) => Some(format!("{LIST_WORD} of {}", canonical_spec_word(element))),
			None if crate::traits::is_builtin_trait(word) || shadowed.types.contains(word) => Some(word.to_string()),
			None => type_word_kind(word).map(|_| canonical_spec_word(word).to_string()),
		},
		_ => None,
	}
}

fn symbol_words(nodes: &[Node]) -> Option<Vec<&str>> {
	nodes.iter().map(|node| match node.drop_meta() {
		Node::Symbol(word) => Some(word.as_str()),
		_ => None,
	}).collect()
}

/// Meta key marking a type word compared with `==` (not `is`): only `is` tests types (user decision #30)
const EQUALITY_OPERAND: &str = "equality operand";

/// The right side of `x == word` as the parser marks it when the word names a type
pub fn equality_operand(word: Node) -> Node {
	match word.drop_meta() {
		Node::Symbol(name) if type_spec(&[name.as_str()], &Names::default()).is_some() => {
			Node::Meta { node: Box::new(word), data: Box::new(Node::key(EQUALITY_OPERAND, Node::True)) }
		}
		_ => word,
	}
}

/// Meta key on the name compared in `x is v` (set by the parser): v as written, before it is lowered (`100 times [0]`), for the
/// error that teaches `x be v` when x is defined nowhere (P61, wasm_emitter emit_undefined_comparison)
pub const COMPARED_WITH: &str = "compared with";

pub fn with_compared_text(subject: Node, written: &str) -> Node {
	match subject.drop_meta() {
		Node::Symbol(_) => Node::Meta { node: Box::new(subject), data: Box::new(Node::key(COMPARED_WITH, text(written))) },
		_ => subject,
	}
}

/// The written value a compared name carries (with_compared_text)
pub fn compared_text(subject: &Node) -> Option<String> {
	match subject {
		Node::Meta { node, data } => match data.as_ref() {
			Node::Key(key, _, value) if key.name() == COMPARED_WITH => Some(value.drop_meta().name()),
			_ => compared_text(node),
		},
		_ => None,
	}
}

pub fn is_equality_operand(node: &Node) -> bool {
	matches!(node, Node::Meta { data, .. } if matches!(data.as_ref(), Node::Key(key, _, _) if key.name() == EQUALITY_OPERAND))
}

/// `x is int`; `type(x) is int` tests x, as the type of x is what it names (card type-int)
fn is_type_call(subject: Node, spec: String) -> Node {
	let tested = typed_argument(&subject).unwrap_or(subject);
	call(IS_TYPE, vec![tested, Node::Text(spec)])
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
		Node::List(items, bracket, separator) => {
			if let Some(call) = type_of_word(&items, shadowed).or_else(|| spaced_type_test(&items, shadowed)) {
				return expand(call, shadowed);
			}
			Node::List(items.into_iter().map(|item| expand(item, shadowed)).collect(), bracket, separator)
		}
		Node::Key(subject, Op::Eq, right) => match symbol_words(std::slice::from_ref(&*right)).and_then(|words| type_spec(&words, shadowed)) {
			// `x == int` compares x with the type value int: equal only to that type (wasm_emitter emit_structural_equality)
			Some(spec) if !is_equality_operand(&right) => is_type_call(expand(*subject, shadowed), spec),
			_ => key(expand(*subject, shadowed), Op::Eq, expand(*right, shadowed)),
		},
		Node::Key(left, op, right) => key(expand(*left, shadowed), op, expand(*right, shadowed)),
		Node::Meta { node, data } => Node::Meta { node: Box::new(expand(*node, shadowed)), data },
		other => other,
	}
}

/// `x is a number` is the items `x is a` and `number`: the words after the comparison continue the type phrase
fn spaced_type_test(items: &[Node], shadowed: &Names) -> Option<Node> {
	let [first, tail @ ..] = items else { return None };
	let Node::Key(subject, Op::Eq, right) = first.drop_meta() else { return None };
	let Node::Symbol(word) = right.drop_meta() else { return None };
	let mut words = vec![word.as_str()];
	words.extend(symbol_words(tail)?);
	Some(is_type_call(subject.as_ref().clone(), type_spec(&words, shadowed)?))
}

/// `type of x`: the parser reads `type of` as a declaration head, its argument is the next item
fn type_of_word(items: &[Node], shadowed: &Names) -> Option<Node> {
	let [head, argument @ ..] = items else { return None };
	if argument.is_empty() || shadowed.contains(TYPE_WORD) {
		return None;
	}
	let Node::Type { name, body } = head.drop_meta() else { return None };
	if !name.is_symbol(OF_WORD) || !matches!(body.drop_meta(), Node::Empty) {
		return None;
	}
	let argument = match argument {
		[single] => single.clone(),
		many => Node::List(many.to_vec(), Bracket::None, Separator::Space),
	};
	Some(call(TYPE_WORD, vec![argument]))
}
