//! Other spellings of the host words, renamed to the host word unless the program defines them
use super::*;

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
/// `random(a, b)` (decision random-forms): a whole number from a to b for whole numbers, the prelude word
const RANDOM_BETWEEN: &str = "random_between";
const RANDOM_WORD: &str = "random";

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
	let node = if defined.contains(RANDOM_WORD) { node } else { random_forms(node) };
	renamed_heads(node, &aliases)
}

/// `random int`, `random(bool)`: the call of the prelude word giving a random value of that type, a type without one an
/// error naming the forms random has; `random(1, 6)` the prelude's random_between. `random float` is `random()`
fn random_forms(node: Node) -> Node {
	let node = children_rewritten(node, random_forms);
	let Node::List(items, Bracket::Round | Bracket::None, _) = node.drop_meta() else { return node };
	let Some((_, arguments)) = items.split_first().filter(|(random, _)| random.is_symbol(RANDOM_WORD)) else {
		return node;
	};
	let prelude_call = |word: &str, arguments: &[Node]| {
		let word = crate::modules::prelude_name(word).unwrap_or_else(|| word.to_string());
		crate::lowering::nodes::call(&word, arguments.to_vec())
	};
	match arguments {
		[_, _] => prelude_call(RANDOM_BETWEEN, arguments),
		[type_word] => match type_word.symbol_name().and_then(crate::type_tests::builtin_type_name) {
			Some(type_name) => match RANDOM_OF_TYPES.iter().find(|(name, _)| *name == type_name) {
				Some((_, word)) => prelude_call(word, &[]),
				None => crate::node::error(&format!(
					"random {type_name}: a {type_name} has no random value; random int, random bool, random codepoint, \
					 random float, random(n) for 0…n-1 or random(a, b) for a to b"
				)),
			},
			None => node,
		},
		_ => node,
	}
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
