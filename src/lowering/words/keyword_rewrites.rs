//! Keyword-triggered local rewrites (user decision pass-triggers): a rewrite of one node, triggered by a word of the
//! node's own (has_own_word: one of its items, a key's operator, a declaration head's name), is an
//! entry of a table instead of a pass of its own. One bottom-up walk applies a whole table at its place in the pass
//! lists (src/pipeline.rs) and skips a program holding none of the table's words. A fact of the whole program an entry
//! reads (its bindings) is computed once, before the walk, and handed in; a rewrite that needs another place in the
//! order stays a pass. Twin: lib/compiler/scope.warp keyword_rewrites.

use crate::node::{Bracket, Node, Separator};
use crate::program_names::Bindings;

/// A rewrite of one node whose own words hold one `triggered_by` accepts
pub struct KeywordRewrite {
	pub triggered_by: fn(&str) -> bool,
	pub rewrite: fn(Node, &mut Facts) -> Node,
	/// reads the program's bindings (Facts::bindings)
	pub reads_bindings: bool,
}

/// What a rewrite reads besides its node: `fresh` numbers the names it makes, from 0 per walk (`times·1`)
pub struct Facts {
	pub fresh: usize,
	bindings: Option<Bindings>,
}

impl Facts {
	/// The names the program binds, as it was before the walk
	pub fn bindings(&self) -> &Bindings {
		self.bindings.as_ref().expect("an entry reading the bindings sets reads_bindings")
	}
}

/// `program` with each node rewritten bottom-up by the entries of `table` its own words trigger, in table order
pub fn rewritten(program: Node, table: &[KeywordRewrite]) -> Node {
	let triggered: Vec<&KeywordRewrite> =
		table.iter().filter(|rewrite| mentions_own_word(&program, rewrite.triggered_by)).collect();
	if triggered.is_empty() {
		return program;
	}
	let bindings = triggered.iter().any(|rewrite| rewrite.reads_bindings).then(|| Bindings::of(&program));
	walk(program, &triggered, &mut Facts { fresh: 0, bindings })
}

/// The entries see a node without its Meta layers, which stay around the result
fn walk(node: Node, triggered: &[&KeywordRewrite], facts: &mut Facts) -> Node {
	node.under_meta(|node| {
		let node = node.map_children(|child| walk(child, triggered, facts));
		triggered.iter().fold(node, |node, rewrite| match has_own_word(&node, rewrite.triggered_by) {
			true => (rewrite.rewrite)(node, facts),
			false => node,
		})
	})
}

fn mentions_own_word(program: &Node, triggered_by: fn(&str) -> bool) -> bool {
	let mut found = false;
	program.visit(&mut |node| {
		found |= match node {
			Node::Symbol(word) => triggered_by(word),
			Node::Key(_, op, _) => triggered_by(op.as_str()),
			Node::Type { name, .. } => is_word(name, triggered_by),
			_ => false,
		}
	});
	found
}

/// A node's own words: its items' words, a key's operator, a declaration head's name (`type of`)
fn has_own_word(node: &Node, triggered_by: fn(&str) -> bool) -> bool {
	match node {
		Node::List(items, _, _) => items.iter().any(|item| is_word_of_item(item, triggered_by)),
		Node::Key(_, op, _) => triggered_by(op.as_str()),
		Node::Type { name, .. } => is_word(name, triggered_by),
		_ => false,
	}
}

/// An item's words: the word itself, a declaration head's name, the words of a phrase the parser grouped inside the
/// phrase (`x = sum of xs` arrives as `sum (of xs)`, nodes.rs phrase_words)
fn is_word_of_item(item: &Node, triggered_by: fn(&str) -> bool) -> bool {
	match item.drop_meta() {
		Node::Type { name, .. } => is_word(name, triggered_by),
		Node::List(words, Bracket::None, Separator::Space) => {
			words.iter().any(|word| is_word_of_item(word, triggered_by))
		}
		word => is_word(word, triggered_by),
	}
}

fn is_word(node: &Node, triggered_by: fn(&str) -> bool) -> bool {
	node.drop_meta().symbol_name().is_some_and(|word| triggered_by(word))
}
