//! `count x in y`, `count(y, x)` and `y.count(x)`: how often x occurs in y
use super::*;

/// The pass of `count x in y` (count_in), before the lambdas its rewrite uses are lowered and before `x in y` is membership;
/// also `y.count(x)` and `count(y, x)` (card banana-count) unless the program defines its own count
pub fn lower_count_in(node: Node) -> Node {
	let library_count = !defines_count(std::slice::from_ref(&node));
	lower_counts(node, library_count)
}

/// The prelude words the program's counts call (count_item, count_of), for modules::resolve, which runs before this
/// pass: a program that only mentions `count` loads neither
pub(crate) fn implied_count_words(statements: &[Node]) -> Vec<String> {
	let library_count = !defines_count(statements);
	let mut words = Vec::new();
	for statement in statements {
		statement.visit(&mut |node| {
			// `[count v in s for v in vowels]`: comprehensions::lower, a later pass, makes its element `count v in s`
			let element = match node {
				Node::List(items, _, _) => items.iter().position(starts_a_loop).and_then(|at| count_in(&items[..at])),
				_ => None,
			};
			if let Some(Node::List(call, _, _)) = counted(node, library_count).or(element) {
				words.extend(call.first().map(Node::name));
			}
		});
	}
	words
}

/// `for v in vowels` after a comprehension's element
fn starts_a_loop(item: &Node) -> bool {
	match item.drop_meta() {
		Node::List(parts, _, _) => parts.first().is_some_and(|first| first.is_symbol(FOR_WORD)),
		other => other.is_symbol(FOR_WORD),
	}
}

fn defines_count(statements: &[Node]) -> bool {
	let mut found = Vec::new();
	statements.iter().for_each(|statement| crate::function_values::definitions(statement, &mut found));
	found.iter().any(|definition| definition.name == COUNT_WORD)
}

/// The call a count form is (`count x in y`, `count(y, x)`, `y.count(x)`), none for any other node
fn counted(node: &Node, library_count: bool) -> Option<Node> {
	match node {
		Node::List(items, bracket, _) => {
			count_in(items).or_else(|| library_count.then(|| count_call(items, bracket)).flatten())
		}
		Node::Key(receiver, Op::Dot, call) if library_count => count_method(receiver, call),
		_ => None,
	}
}

pub(super) fn lower_counts(node: Node, library_count: bool) -> Node {
	if let Some(occurrences) = counted(&node, library_count) {
		return lower_counts(occurrences, library_count);
	}
	children_rewritten(node, |child| lower_counts(child, library_count))
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
	Some(prelude_call(if substring { COUNT_PART } else { COUNT_ITEM }, vec![haystack, needle.clone()]))
}
