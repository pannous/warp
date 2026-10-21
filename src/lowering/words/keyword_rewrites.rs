//! Keyword-triggered local rewrites (user decision pass-triggers): a rewrite of one node, triggered by a word among the
//! node's own items, is an entry of a table instead of a pass of its own. One bottom-up walk applies a whole table at
//! its place in the pass lists (src/pipeline.rs) and skips a program holding none of the table's words. A rewrite that
//! needs a fact of the whole program, or another place in the order, stays a pass. Twin: lib/compiler/walk.warp
//! keyword_rewrites.

use crate::node::Node;

/// A rewrite of one node whose own items hold a word `triggered_by` accepts; `fresh` numbers the names it makes, from
/// 0 per walk (`times·1`)
pub struct KeywordRewrite {
	pub triggered_by: fn(&str) -> bool,
	pub rewrite: fn(Node, &mut usize) -> Node,
}

/// `program` with each node rewritten bottom-up by the entries of `table` its own words trigger, in table order
pub fn rewritten(program: Node, table: &[KeywordRewrite]) -> Node {
	let triggered: Vec<&KeywordRewrite> =
		table.iter().filter(|rewrite| program.mentions_where(rewrite.triggered_by)).collect();
	if triggered.is_empty() {
		return program;
	}
	let mut fresh = 0;
	walk(program, &triggered, &mut fresh)
}

fn walk(node: Node, triggered: &[&KeywordRewrite], fresh: &mut usize) -> Node {
	let node = node.map_children(|child| walk(child, triggered, fresh));
	triggered.iter().fold(node, |node, rewrite| match has_own_word(&node, rewrite.triggered_by) {
		true => (rewrite.rewrite)(node, fresh),
		false => node,
	})
}

fn has_own_word(node: &Node, triggered_by: fn(&str) -> bool) -> bool {
	let Node::List(items, _, _) = node.drop_meta() else {
		return false;
	};
	items.iter().any(|item| item.drop_meta().symbol_name().is_some_and(|word| triggered_by(word)))
}
