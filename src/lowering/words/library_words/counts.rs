//! `count x in y`, `count(y, x)` and `y.count(x)`: how often x occurs in y
use super::*;

/// The pass of `count x in y` (count_in), before the lambdas its rewrite uses are lowered and before `x in y` is membership;
/// also `y.count(x)` and `count(y, x)` (card banana-count) unless the program defines its own count
pub fn lower_count_in(node: Node) -> Node {
	let mut found = Vec::new();
	crate::function_values::definitions(&node, &mut found);
	let library_count = !found.iter().any(|definition| definition.name == COUNT_WORD);
	lower_counts(node, library_count)
}

pub(super) fn lower_counts(node: Node, library_count: bool) -> Node {
	let counted = |node: &Node| match node {
		Node::List(items, bracket, _) => {
			let library_calls = || count_of_call(items, bracket).or_else(|| count_call(items, bracket));
			count_in(items).or_else(|| library_count.then(library_calls).flatten())
		}
		Node::Key(receiver, Op::Dot, call) if library_count => count_method(receiver, call),
		_ => None,
	};
	if let Some(occurrences) = counted(&node) {
		return lower_counts(occurrences, library_count);
	}
	children_rewritten(node, |child| lower_counts(child, library_count))
}

/// `count(reverse xs)`, `count sorted xs`: the count of the library word's call, `count(reverse(xs))`
fn count_of_call(items: &[Node], bracket: &Bracket) -> Option<Node> {
	let [count, word, arguments @ ..] = items else { return None };
	if !count.is_symbol(COUNT_WORD)
		|| !matches!(word.drop_meta(), Node::Symbol(name) if is_library_word(name))
		|| arguments.is_empty()
	{
		return None;
	}
	let counted = Node::List(items[1..].to_vec(), Bracket::None, Separator::Space);
	Some(Node::List(vec![count.clone(), counted], bracket.clone(), Separator::None))
}

/// `count(y, x)` is `count x in y`
pub(super) fn count_call(items: &[Node], bracket: &Bracket) -> Option<Node> {
	let [count, haystack, needle] = items else { return None };
	(*bracket == Bracket::Round && count.is_symbol(COUNT_WORD)).then(|| counted_in(count, needle, haystack)).flatten()
}

/// `y.count(x)` is `count x in y`
pub(super) fn count_method(receiver: &Node, call: &Node) -> Option<Node> {
	let Node::List(items, _, _) = call.drop_meta() else {
		return None;
	};
	let [count, needle] = items.as_slice() else { return None };
	count.is_symbol(COUNT_WORD).then(|| counted_in(count, needle, receiver)).flatten()
}

pub(super) fn counted_in(count: &Node, needle: &Node, haystack: &Node) -> Option<Node> {
	count_in(&[count.clone(), needle.clone(), symbol(IN_WORD), haystack.clone()])
}

/// `count x in y`: how often x occurs in y. A text of several characters counts as a substring of a text (`count "an"
/// in "banana"` → 2), anything else as an item of a list or a character of a text; `count bytes in t` is `t.bytes`
pub(super) fn count_in(items: &[Node]) -> Option<Node> {
	// `count 'a' in s` arrives as the words or as `count ('a' in s)`
	let phrase: Vec<Node> = match items {
		[count, phrase] => match phrase.drop_meta() {
			Node::List(rest, Bracket::None, _) => [vec![count.clone()], rest.clone()].concat(),
			_ => return None,
		},
		_ => items.to_vec(),
	};
	let [count, needle, in_word, haystack @ ..] = phrase.as_slice() else {
		return None;
	};
	if !count.is_symbol(COUNT_WORD) || !in_word.is_symbol(IN_WORD) || haystack.is_empty() {
		return None;
	}
	let haystack = match haystack {
		[single] => single.clone(),
		several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
	};
	if let Some(unit) =
		matches!(needle.drop_meta(), Node::Symbol(_)).then(|| crate::analyzer::text_unit(&needle.name())).flatten()
	{
		return Some(key(haystack, Op::Dot, symbol(unit)));
	}
	let substring = matches!(needle.drop_meta(), Node::Text(text) if text.chars().count() > 1)
		&& !matches!(haystack.drop_meta(), Node::List(_, Bracket::Square, _));
	let template = if substring { COUNT_SUBSTRING_TEMPLATE } else { COUNT_ITEM_TEMPLATE };
	Some(substitute(substitute(parse(template), COUNT_HAYSTACK, &haystack), COUNT_NEEDLE, needle))
}
