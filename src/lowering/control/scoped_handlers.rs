//! Block-scoped event handlers `on ask { 42 } in { compute() }` were dropped (user, cleanup-cuts, card
//! cleanup-drop-scoped-handlers, notes/scoped_handlers.md): the form is an error naming the program-wide `on ask {…}`

use super::nodes::{children_rewritten, phrase_words};
use crate::errors::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::words::{IN_WORD, ON_WORD};

pub fn lower(program: Node) -> Node {
	match scoped_handler_event(&program) {
		Some(event) => Diagnostic::at(
			&program,
			format!("`on {event} {{…}} in {{…}}`: block-scoped handlers were dropped; a program-wide `on {event} {{…}}` answers every `emit {event}`"),
		)
		.into_error(),
		None => children_rewritten(program, lower),
	}
}

/// The event of `on ask {body} in {block}`
fn scoped_handler_event(node: &Node) -> Option<String> {
	let Node::List(items, _, Separator::Space) = node.drop_meta() else {
		return None;
	};
	let items = phrase_words(items);
	let [on, words @ .., body, in_word, block] = items.as_slice() else {
		return None;
	};
	let curly = |node: &Node| matches!(node.drop_meta(), Node::List(_, Bracket::Curly, _));
	let words: Vec<String> = words.iter().map(crate::declarations::word).collect();
	let is_scoped = crate::declarations::word(on) == ON_WORD
		&& crate::declarations::word(in_word) == IN_WORD
		&& curly(body)
		&& curly(block)
		&& !words.is_empty()
		&& !words.iter().any(String::is_empty);
	is_scoped.then(|| words.join(" "))
}
