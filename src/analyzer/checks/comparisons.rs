//! Comparisons: a `> 100` outside a match arm compares nothing
use super::*;

/// Booleans are not numbers: `true + true` is rejected, not 2 (the runtime still encodes them as Int 1/0)
/// `> 100` outside a match arm compares nothing (card leading-gt: it was dropped silently, leaving 100)
pub(crate) fn check_subjectless_comparison(node: &Node) -> Option<Diagnostic> {
	match node.drop_meta() {
		Node::Key(left, op, right) if op.is_ordering() && left.is_empty_node() => {
			let written = format!("{} {}", op.as_str(), right.serialize());
			Some(Diagnostic::at(
				node,
				format!(
					"`{written}` compares nothing: write what is compared, `x {written}` (in a match arm it compares the subject)"
				),
			))
		}
		Node::Key(left, _, right) => check_subjectless_comparison(left).or_else(|| check_subjectless_comparison(right)),
		Node::List(items, _, _) => items.iter().find_map(check_subjectless_comparison),
		_ => None,
	}
}
