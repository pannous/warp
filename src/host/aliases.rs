//! Other spellings of the host words, renamed to the host word unless the program defines them
use super::*;
use crate::operators::Op;

/// Other spellings of the host words, for any number of arguments or only for the given one (user decision #14e:
/// `download <url>`, `curl <url>` and `wget <url>` are `fetch <url>` (card unix-words); `random(n)` is `random_below(n)`)
pub(crate) const HOST_ALIASES: [(&str, Option<usize>, &str); 4] = [
	("download", None, "fetch"),
	("curl", None, "fetch"),
	("wget", None, "fetch"),
	(RANDOM_WORD, Some(1), RANDOM_BELOW),
];

/// `random T` (decision random-forms): a random value of the type T, by the prelude word for it
const RANDOM_OF_TYPES: [(&str, &str); 4] =
	[("int", "random_int"), ("bool", "random_bool"), ("codepoint", "random_codepoint"), ("float", RANDOM)];
/// `random(a, b)` (decision random-forms): a whole number from a to b for whole numbers, the prelude word; `random a..b`
/// one up to b, and of a declared float a float from a up to b
const RANDOM_BETWEEN: &str = "random_between";
const RANDOM_UNTIL: &str = "random_until";
const RANDOM_FRACTION: &str = "random_fraction";
const RANDOM_WORD: &str = "random";
const RANDOM_ELEMENT: &str = "random_element";
const RANDOM_OF: &str = "random_of";

/// `download url` → `fetch url`: an alias at the head of an application or call, unless the program defines the alias itself
pub fn lower_aliases(node: Node) -> Node {
	if !node.mentions_any(&HOST_ALIASES.map(|(alias, _, _)| alias)) {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	let aliases: Vec<_> = HOST_ALIASES.into_iter().filter(|(alias, _, _)| !defined.contains(*alias)).collect();
	if aliases.is_empty() {
		return node;
	}
	renamed_heads(node, &aliases)
}

/// The random forms as the prelude words they call, before modules::resolve: a program loads only the words it uses
pub fn lower_random_forms(node: Node) -> Node {
	if !node.mentions_any(&[RANDOM_WORD]) || crate::library_words::defined_names(&node).contains(RANDOM_WORD) {
		return node;
	}
	random_forms(node)
}

/// `random int`, `random(bool)`: the call of the prelude word giving a random value of that type, a type without one an
/// error naming the forms random has; `random(1, 6)` and `random 1...6` the prelude's random_between, `random 1..6` one
/// of the range's elements (1 to 5). `random float` is `random()`. The declared type of the target picks the result:
/// `float z = random 1..10` is a float from 1 up to 10
fn random_forms(node: Node) -> Node {
	if let Some(fraction) = declared_float_random(&node) {
		return fraction;
	}
	let node = children_rewritten(node, random_forms);
	match comprehension_head(node) {
		Ok((head, rest, bracket, separator)) => {
			Node::List([vec![random_call(head)], rest].concat(), bracket, separator)
		}
		Err(node) => random_call(node),
	}
}

/// `[random 1..4 for i in 1 to 9]` as parsed, its words flat and the `for` clause a list: the call `random 1..4`, the
/// rest from `for` on
fn comprehension_head(node: Node) -> Result<(Node, Vec<Node>, Bracket, Separator), Node> {
	let Node::List(items, bracket @ (Bracket::Square | Bracket::None), separator) = node else { return Err(node) };
	let starts_for = |item: &Node| match item.drop_meta() {
		Node::List(clause, _, _) => clause.first().is_some_and(|word| word.is_symbol(crate::words::FOR_WORD)),
		word => word.is_symbol(crate::words::FOR_WORD),
	};
	let for_at = items.iter().position(starts_for);
	match for_at {
		Some(at) if at > 1 && items[0].is_symbol(RANDOM_WORD) => {
			let mut head = items;
			let rest = head.split_off(at);
			Ok((Node::List(head, Bracket::None, Separator::Space), rest, bracket, separator))
		}
		_ => Err(Node::List(items, bracket, separator)),
	}
}

fn random_call(node: Node) -> Node {
	if let Some(compared) = random_compared(&node) {
		return compared;
	}
	let Some(arguments) = random_arguments(&node) else { return node };
	if let Some((low, high, ends)) = random_ends(arguments) {
		let word = if ends == Op::Range { RANDOM_UNTIL } else { RANDOM_BETWEEN };
		return prelude_call(word, vec![low, high]);
	}
	match arguments {
		[type_word] => match type_word.symbol_name().and_then(crate::type_tests::builtin_type_name) {
			Some(type_name) => match RANDOM_OF_TYPES.iter().find(|(name, _)| *name == type_name) {
				Some((_, word)) => prelude_call(word, vec![]),
				None => crate::node::error(&format!(
					"random {type_name}: a {type_name} has no random value; random int, random bool, random codepoint, \
					 random float, random(n) for 0…n-1 or random(a, b) for a to b"
				)),
			},
			None => random_of_value(type_word).unwrap_or(node),
		},
		_ => node,
	}
}

/// `random [a b c]` one of the elements, `random 2.5` a float from 0 up to 2.5, `random xs` by its value's type at run
/// time (card random-dispatches); `random 6` stays the host's random_below
fn random_of_value(argument: &Node) -> Option<Node> {
	match argument.drop_meta() {
		Node::Number(number) if number.is_integer() => None,
		Node::Number(_) => Some(prelude_call(RANDOM_FRACTION, vec![Node::Number(Number::Int(0)), argument.clone()])),
		Node::List(_, Bracket::Square, _) => Some(prelude_call(RANDOM_ELEMENT, vec![argument.clone()])),
		_ => Some(prelude_call(RANDOM_OF, vec![argument.clone()])),
	}
}

/// `random 6 < 6` is (random 6) < 6: random never takes a truth value, so a comparison or logical chain after it
/// compares its result, as nest_prefix_calls groups a function whose first parameter rules out one (`random(6 < 6)`
/// written with parentheses stays the call)
fn random_compared(node: &Node) -> Option<Node> {
	let Node::List(items, Bracket::None, separator) = node.drop_meta() else { return None };
	let [random, argument] = items.as_slice() else { return None };
	let Node::Key(left, op, right) = argument.drop_meta() else { return None };
	if !random.is_symbol(RANDOM_WORD) || !(op.is_relation() || matches!(op, Op::And | Op::Or)) {
		return None;
	}
	let called = Node::List(vec![random.clone(), left.as_ref().clone()], Bracket::None, separator.clone());
	Some(crate::lowering::nodes::key(random_call(called), op.clone(), right.as_ref().clone()))
}

fn prelude_call(word: &str, arguments: Vec<Node>) -> Node {
	let word = crate::modules::prelude_name(word).unwrap_or_else(|| word.to_string());
	crate::lowering::nodes::call(&word, arguments)
}

/// The arguments of `random …`, `random(…)`
fn random_arguments(node: &Node) -> Option<&[Node]> {
	let Node::List(items, Bracket::Round | Bracket::None, _) = node.drop_meta() else { return None };
	let (random, arguments) = items.split_first()?;
	random.is_symbol(RANDOM_WORD).then_some(arguments)
}

/// The ends of `random(a, b)`, `random a...b` and `random a to b` (Op::To: b included) or `random a..b` (Op::Range)
fn random_ends(arguments: &[Node]) -> Option<(Node, Node, Op)> {
	match arguments {
		[low, high] => Some((low.clone(), high.clone(), Op::To)),
		[range] => match range.drop_meta() {
			Node::Key(low, ends @ (Op::Range | Op::To), high) => Some((*low.clone(), *high.clone(), *ends)),
			_ => None,
		},
		_ => None,
	}
}

/// `float z = random 1..10`, `z: float = random(1, 10)`: a float from 1 up to 10, as the target declares
fn declared_float_random(node: &Node) -> Option<Node> {
	let Node::Key(target, Op::Assign, value) = node.drop_meta() else { return None };
	let (_, type_name) = crate::analyzer::declaring_name_and_type(target.drop_meta())?;
	if crate::type_tests::builtin_type_name(&type_name) != Some(crate::words::FLOAT_WORD) {
		return None;
	}
	let (low, high, _) = random_ends(random_arguments(value)?)?;
	let fraction = prelude_call(RANDOM_FRACTION, vec![low, high]);
	Some(Node::Key(target.clone(), Op::Assign, Box::new(fraction)))
}

pub(crate) fn renamed_heads(node: Node, aliases: &[(&str, Option<usize>, &str)]) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let mut items: Vec<Node> = items.into_iter().map(|item| renamed_heads(item, aliases)).collect();
			if let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) {
				let arguments = items.len() - 1;
				if let Some((_, _, word)) = aliases
					.iter()
					.find(|(alias, arity, _)| alias == name && arity.is_none_or(|arity| arity == arguments))
				{
					items[0] = Node::Symbol(word.to_string());
				}
			}
			Node::List(items, bracket, separator)
		}
		other => children_rewritten(other, |child| renamed_heads(child, aliases)),
	}
}
