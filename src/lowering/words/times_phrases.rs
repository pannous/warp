//! `n times …`: the parser keeps the phrase [n times operand] as written (suffixes.rs parse_times_loop), this pass
//! gives it its meaning. A block is a loop with a hidden counter of its own, so nested loops do not meet; `n times [x]`
//! is the list of n x; any other operand is repeated by `times·text` (`3 times "ab"`, `3 times 4` multiplies).
//! `once`, `twice`, `thrice` before a block are its count (`thrice{…}` is `3 times {…}`, card thrice-block), unless
//! the program uses the word otherwise: `twice(x) := x*2` keeps it a function.

use super::nodes::{assign, call, int, is_assigned_data, key, symbol};
use crate::node::{error, Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::{for_in_loop, TEXT_TIMES, TIMES_WORD};

const COUNT_WORDS: [(&str, i64); 3] = [("once", 1), ("twice", 2), ("thrice", 3)];

pub fn lower(node: Node) -> Node {
	let node = COUNT_WORDS.iter().fold(node, |node, &(word, count)| with_count_word(node, word, count));
	if !node.mentions_any(&[TIMES_WORD]) {
		return node;
	}
	let mut loops = 0;
	expand(node, &mut loops)
}

fn expand(node: Node, loops: &mut usize) -> Node {
	let node = node.map_children(|child| expand(child, loops));
	let Node::List(items, Bracket::None, Separator::Space) = node.drop_meta() else {
		return node;
	};
	match items.as_slice() {
		[count, word, operand] if word.is_symbol(TIMES_WORD) => {
			meaning(count.clone(), operand.clone(), loops).with_meta_of(&node)
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

/// `for times·N in 0..count body`; the count is evaluated once, before the first round (user decision #22): the body
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

/// `thrice{…}`, `thrice {…}` as `3 times {…}`, when the program has no other use of the word
fn with_count_word(program: Node, word: &str, count: i64) -> Node {
	if !program.mentions(word) {
		return program;
	}
	let counted = counted_blocks(program.clone(), word, count);
	if counted.mentions(word) {
		return program;
	}
	counted
}

fn counted_blocks(node: Node, word: &str, count: i64) -> Node {
	// `x = {twice: {a: 1}}` is data with the field twice
	if matches!(node.drop_meta(), Node::Key(target, Op::Assign, value) if is_assigned_data(target, value)) {
		return node;
	}
	let node = node.map_children(|child| counted_blocks(child, word, count));
	let block = match node.drop_meta() {
		Node::Key(head, Op::Colon, block) if head.is_symbol(word) => block.as_ref(),
		Node::List(items, Bracket::None, Separator::Space) => match items.as_slice() {
			[head, block] if head.is_symbol(word) => block,
			_ => return node,
		},
		_ => return node,
	};
	if !matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return node;
	}
	Node::List(vec![int(count), symbol(TIMES_WORD), block.clone()], Bracket::None, Separator::Space).with_meta_of(&node)
}
