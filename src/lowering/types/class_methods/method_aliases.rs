//! Other languages' method names on a class that does not define them: a deque's `append`, a set's `contains`
use super::*;

/// Other languages' method names (Python's deque, Java's Deque and Queue, JS's Array and Set) and the warp methods they
/// mean, the first one the class defines taken, with a note; only on a class that does not define the name itself
const METHOD_ALIASES: [(&str, &[&str]); 23] = [
	("clone", &["copy"]), // clone always calls the class's copy (P207)
	("append", &["push_back", "push", "enqueue", "add"]),
	("addLast", &["push_back", "enqueue"]),
	("offerLast", &["push_back", "enqueue"]),
	("offer", &["enqueue", "push_back"]),
	("push", &["push_back", "enqueue"]),
	("appendleft", &["push_front"]),
	("addFirst", &["push_front"]),
	("offerFirst", &["push_front"]),
	("unshift", &["push_front"]),
	("popleft", &["pop_front", "dequeue"]),
	("pollFirst", &["pop_front", "dequeue"]),
	("removeFirst", &["pop_front", "dequeue"]),
	("shift", &["pop_front", "dequeue"]),
	("poll", &["dequeue", "pop_front"]),
	("pollLast", &["pop_back", "pop"]),
	("removeLast", &["pop_back", "pop"]),
	("pop", &["pop_back"]),
	("contains", &["has"]),
	("includes", &["has"]),
	("delete", &["remove"]),
	("discard", &["remove"]),
	("len", &["size"]),
];

/// `d.append(1)` of a deque, `s.contains(2)` of a set, `len(s)`: the class's own method when it does not define that
/// name (METHOD_ALIASES, analyzer::counting_words), with a note
pub(super) fn method_aliases(node: Node) -> Node {
	let mut methods: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
	node.visit(&mut |part| {
		if let Node::Type { name, body } = part {
			methods.insert(
				name.drop_meta().name(),
				class_items(body).iter().filter_map(method_parts).map(|(method, _, _)| method).collect(),
			);
		}
	});
	if methods.is_empty() {
		return node;
	}
	let classes: Vec<String> = methods.keys().cloned().collect();
	let instances: std::collections::HashMap<String, Vec<String>> = instance_classes(&node, &classes)
		.into_iter()
		.map(|(variable, class)| (variable, methods[&class].clone()))
		.collect();
	with_method_aliases(node, &instances)
}

/// The method of the class for `written`: the name itself, or the alias's warp method, or for a size word the size
/// method the class defines
pub(super) fn aliased_method(written: &str, defined: &[String]) -> Option<String> {
	if defined.iter().any(|method| method == written) {
		return None;
	}
	let candidates: Vec<&str> = match crate::analyzer::is_counting_word(written) {
		true => crate::analyzer::counting_words(),
		false => METHOD_ALIASES
			.iter()
			.find(|(alias, _)| *alias == written)
			.map(|(_, methods)| methods.to_vec())
			.unwrap_or_default(),
	};
	let method = candidates.into_iter().find(|candidate| defined.iter().any(|method| method == candidate))?;
	crate::errors::diagnostic::note_alias(written, method);
	Some(method.to_string())
}

fn with_method_aliases(node: Node, instances: &std::collections::HashMap<String, Vec<String>>) -> Node {
	let defined_by = |receiver: &Node| match receiver.drop_meta() {
		Node::Symbol(variable) => instances.get(variable),
		_ => None,
	};
	match node {
		Node::Key(receiver, Op::Dot, member) if defined_by(&receiver).is_some() => {
			let member = match member.drop_meta() {
				Node::List(items, Bracket::Round, separator) if !items.is_empty() => {
					match aliased_method(&items[0].drop_meta().name(), defined_by(&receiver).expect("guarded")) {
						Some(method) => Node::List(
							[
								vec![Node::Symbol(method)],
								items[1..].iter().cloned().map(|item| with_method_aliases(item, instances)).collect(),
							]
							.concat(),
							Bracket::Round,
							separator.clone(),
						),
						None => with_method_aliases(*member, instances),
					}
				}
				_ => *member,
			};
			Node::Key(receiver, Op::Dot, Box::new(member))
		}
		// `len(s)`: `s.size()`; `size(s)` of a class defining size(): `s.size()` too (card size-instance)
		Node::List(items, Bracket::Round, separator) if matches!(items.as_slice(), [word, argument] if crate::analyzer::is_counting_word(&word.drop_meta().name()) && defined_by(argument).is_some()) =>
		{
			let written = items[0].drop_meta().name();
			let defined = defined_by(&items[1]).expect("guarded");
			let method = match defined.contains(&written) {
				true => Some(written),
				false => aliased_method(&written, defined),
			};
			match method {
				Some(method) => key(items[1].clone(), Op::Dot, call(&method, vec![])),
				None => Node::List(items, Bracket::Round, separator),
			}
		}
		other => other.map_children(|child| with_method_aliases(child, instances)),
	}
}
