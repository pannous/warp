//! `to_json(p)` of a class instance, and other languages' serializer calls
use super::*;

/// json's word for a program with classes (lib/json.warp): the classes' names go along, so an instance is its fields
const TO_JSON_WORD: &str = "to_json";
const TO_JSON_OF_CLASSES_WORD: &str = "to_json_of_classes";
/// Other languages' calls giving an instance's json or its fields: Kotlin's `Json.encodeToString(p)` is to_json(p),
/// Python's `dataclasses.asdict(p)` the instance itself (its fields read like a map's entries)
const SERIALIZER_ALIASES: [(&str, &str, &str); 2] =
	[("Json", "encodeToString", TO_JSON_WORD), ("dataclasses", "asdict", "")];

/// `to_json(p)` in a program declaring classes: `to_json_of_classes(p, ["Point", …])`, after std_aliases made
/// `JSON.stringify(p)` and `json.dumps(p)` to_json
pub fn lower_json_classes(node: Node) -> Node {
	let mut classes = vec![];
	node.visit(&mut |part| {
		if let Node::Type { name, .. } = part {
			classes.push(Node::Text(name.drop_meta().name()));
		}
	});
	if classes.is_empty() {
		return node;
	}
	with_json_classes(node, &Node::List(classes, Bracket::Square, Separator::Space))
}

fn with_json_classes(node: Node, classes: &Node) -> Node {
	let serializer = |module: &Node, member: &Node| {
		SERIALIZER_ALIASES.iter().find(|(written_module, written_member, _)| {
			module.drop_meta().name() == *written_module && leading_name(member) == *written_member
		})
	};
	match node {
		Node::Key(module, Op::Dot, member) if serializer(&module, &member).is_some() => {
			let (written_module, written_member, word) = serializer(&module, &member).expect("guarded");
			let Node::List(items, Bracket::Round, _) = member.drop_meta() else {
				return Node::Key(module, Op::Dot, member);
			};
			let argument = items.get(1).cloned().unwrap_or(Node::Empty);
			crate::errors::diagnostic::note_alias(
				&format!("{written_module}.{written_member}"),
				if word.is_empty() { "the instance" } else { word },
			);
			match word.is_empty() {
				true => with_json_classes(argument, classes),
				false => with_json_classes(call(word, vec![argument]), classes),
			}
		}
		Node::List(items, Bracket::Round, separator)
			if items.len() == 2 && items[0].drop_meta().name() == TO_JSON_WORD =>
		{
			let value = with_json_classes(items[1].clone(), classes);
			Node::List(vec![symbol(TO_JSON_OF_CLASSES_WORD), value, classes.clone()], Bracket::Round, separator)
		}
		other => other.map_children(|child| with_json_classes(child, classes)),
	}
}
