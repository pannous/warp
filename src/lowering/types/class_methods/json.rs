//! `to_json(p)` of a class instance, and other languages' serializer calls
use super::*;

/// json's word for a program with classes (lib/json.warp): the classes' names go along, so an instance is its fields
const TO_JSON_WORD: &str = "to_json";
const TO_JSON_OF_CLASSES_WORD: &str = "to_json_of_classes";
/// json's word for to_json under `use strict` (decision json-exactness): a fraction is an error, not a float
const EXACT_JSON_WORD: &str = "exact_json";
const USE_WORD: &str = "use";
const JSON_MODULE: &str = "json";
/// Other languages' calls giving an instance's json or its fields: Kotlin's `Json.encodeToString(p)` is to_json(p),
/// Python's `dataclasses.asdict(p)` the instance itself (its fields read like a map's entries)
const SERIALIZER_ALIASES: [(&str, &str, &str); 2] =
	[("Json", "encodeToString", TO_JSON_WORD), ("dataclasses", "asdict", "")];

/// `to_json(p)` in a program declaring classes: `to_json_of_classes(p, ["Point", …])`, under `use strict`
/// `exact_json(p, [classes…])`, after std_aliases made `JSON.stringify(p)` and `json.dumps(p)` to_json
pub fn lower_json_calls(node: Node) -> Node {
	let mut classes = vec![];
	node.visit(&mut |part| {
		if let Node::Type { name, .. } = part {
			classes.push(Node::Text(name.drop_meta().name()));
		}
	});
	// a program without `use json` keeps to_json, so the error naming the missing module names the word written
	let strict =
		crate::errors::diagnostic::warning_mode().fails(crate::errors::diagnostic::Level::Normal) && uses_json(&node);
	let word = match strict {
		true => EXACT_JSON_WORD,
		false if classes.is_empty() => return node,
		false => TO_JSON_OF_CLASSES_WORD,
	};
	with_json_classes(node, &Node::List(classes, Bracket::Square, Separator::Space), word)
}

fn with_json_classes(node: Node, classes: &Node, json_word: &str) -> Node {
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
				true => with_json_classes(argument, classes, json_word),
				false => with_json_classes(call(word, vec![argument]), classes, json_word),
			}
		}
		Node::List(items, Bracket::Round, separator)
			if items.len() == 2 && items[0].drop_meta().name() == TO_JSON_WORD =>
		{
			let value = with_json_classes(items[1].clone(), classes, json_word);
			Node::List(vec![symbol(json_word), value, classes.clone()], Bracket::Round, separator)
		}
		other => other.map_children(|child| with_json_classes(child, classes, json_word)),
	}
}

/// Whether the program says `use json` (std_aliases writes it for `x.json` and JSON.stringify)
fn uses_json(program: &Node) -> bool {
	let mut found = false;
	program.visit(&mut |part| {
		if let Node::List(items, ..) = part.drop_meta() {
			found |= items.first().is_some_and(|word| word.drop_meta().name() == USE_WORD)
				&& items.iter().any(|item| item.drop_meta().name() == JSON_MODULE);
		}
	});
	found
}
