//! Other spellings of the host words, renamed to the host word unless the program defines them
use super::*;

/// Other spellings of the host words, for any number of arguments or only for the given one (user decision #14e:
/// `download <url>`, `curl <url>` and `wget <url>` are `fetch <url>` (card unix-words); `random(n)` is `random_below(n)`)
pub(crate) const HOST_ALIASES: [(&str, Option<usize>, &str); 4] =
	[("download", None, "fetch"), ("curl", None, "fetch"), ("wget", None, "fetch"), ("random", Some(1), RANDOM_BELOW)];

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
