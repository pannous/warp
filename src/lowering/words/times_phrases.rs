//! `n times …`: the parser keeps the phrase [n times operand] as written (suffixes.rs parse_times_loop), this keyword
//! rewrite gives it its meaning. A block is a loop with a hidden counter of its own, so nested loops do not meet;
//! `n times [x]` is the list of n x; any other operand is repeated by `times·text` (`3 times "ab"`, `3 times 4`
//! multiplies).

use super::keyword_rewrites::{Facts, KeywordRewrite};
use super::nodes::{assign, call, int, key, symbol};
use crate::node::{error, Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::{for_in_loop, TEXT_TIMES, TIMES_WORD};

/// The keyword rewrite of the phrase
const REWRITE: KeywordRewrite =
	KeywordRewrite { triggered_by: |word| word == TIMES_WORD, rewrite: phrase, reads_bindings: false };

/// A walk of its own, outside src/pipeline.rs SUGAR_PHRASES: the warp compiler has no times twin (card
/// phrases-out-of-core), so the Rust rewrite runs in the self-hosted front lane too
pub fn lower(program: Node) -> Node {
	super::keyword_rewrites::rewritten(program, &[REWRITE])
}

fn phrase(node: Node, facts: &mut Facts) -> Node {
	let Node::List(items, Bracket::None, Separator::Space) = node.drop_meta() else {
		return node;
	};
	match items.as_slice() {
		[count, word, operand] if word.is_symbol(TIMES_WORD) => {
			meaning(count.clone(), operand.clone(), &mut facts.fresh).with_meta_of(&node)
		}
		_ => node,
	}
}

fn meaning(count: Node, operand: Node, loops: &mut usize) -> Node {
	match operand.drop_meta() {
		Node::List(_, Bracket::Curly, _) | Node::Empty => {
			*loops += 1;
			rounds(count, operand, *loops)
		}
		Node::List(_, Bracket::Square, _) => crate::filled_lists::filled_list(count, &operand)
			.unwrap_or_else(|| error("`n times [x]` repeats one element: `3 times [0]`")),
		_ => call(TEXT_TIMES, vec![count, operand]),
	}
}

/// `for times·N in 0..count body`; the count is evaluated once, before the first round : the body
/// may change what it reads
fn rounds(count: Node, body: Node, number: usize) -> Node {
	let counter = symbol(&format!("{TIMES_WORD}·{number}"));
	let zero_to = |count| key(int(0), Op::Range, count);
	if matches!(count.drop_meta(), Node::Number(_)) {
		return for_in_loop(counter, zero_to(count), body);
	}
	let held = symbol(&format!("{TIMES_WORD}·count·{number}"));
	Node::List(
		vec![assign(held.clone(), count), for_in_loop(counter, zero_to(held), body)],
		Bracket::Round,
		Separator::Semicolon,
	)
}
