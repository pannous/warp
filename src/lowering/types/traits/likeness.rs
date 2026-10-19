//! Likeness: `image like photo` accepts values of one declared type where another is expected; data with the fields of a declared type
use super::*;

/// `image like photo`: values of `kind` are accepted wherever `like` is expected
#[derive(Clone, Debug, PartialEq)]
pub struct Likeness {
	pub kind: String,
	pub like: String,
}

/// `image like photo` written as a statement
pub(super) fn like_declaration(node: &Node) -> Option<Likeness> {
	let Node::List(items, _, Separator::Space) = node.drop_meta() else {
		return None;
	};
	let [Node::Symbol(kind), Node::Symbol(word), Node::Symbol(like)] =
		items.iter().map(Node::drop_meta).collect::<Vec<_>>()[..]
	else {
		return None;
	};
	(word == LIKE_WORD).then(|| Likeness { kind: kind.clone(), like: like.clone() })
}

/// A likeness naming something that is no declared type: the error of the program
pub(super) fn undeclared_likeness(likenesses: &[Likeness], registry: &TypeRegistry) -> Option<Node> {
	let undeclared = likenesses
		.iter()
		.flat_map(|likeness| [&likeness.kind, &likeness.like])
		.find(|name| registry.get_by_name(name).is_none())?;
	Some(crate::node::error(&LIKE_OF_UNDECLARED.replace("NAME", undeclared)))
}

/// Written data given for a declared type, without its ad hoc name: `pic{width:3}` (no `class pic`) and `{width:3}` are the
/// data `{width:3}`, the name is only a label (user, 2026-10-03)
pub(super) fn written_data(value: &Node, registry: &TypeRegistry) -> Option<Node> {
	if let Node::Key(label, Op::Colon, data) = value.drop_meta() {
		if matches!(label.drop_meta(), Node::Symbol(name) if registry.get_by_name(name).is_none()) && data.is_block() {
			return Some(data.as_ref().clone());
		}
	}
	crate::library_words::object_entries(value).map(|_| value.clone())
}

/// The warnings for data whose fields differ from the declared type's: a required field it lacks, a field it has extra
pub(super) fn field_warnings(value: &Node, data: &Node, declared: &str, registry: &TypeRegistry) -> Vec<Diagnostic> {
	let Some(type_def) = registry.get_by_name(declared) else {
		return vec![];
	};
	let entries = crate::library_words::object_entries(data).unwrap_or_default();
	let given: Vec<String> = entries
		.into_iter()
		.map(|(name, _)| name)
		.filter(|name| !name.starts_with(crate::node::ATTRIBUTE_MARK))
		.collect();
	let missing: Vec<String> = type_def
		.fields
		.iter()
		.filter(|field| registry.is_required(type_def, field) && !given.contains(&field.name))
		.map(|field| field.name.clone())
		.collect();
	let extra: Vec<String> =
		given.iter().filter(|name| !type_def.fields.iter().any(|field| field.name == **name)).cloned().collect();
	let warning = |fields: &[String], message: &str, fix: &str| {
		let (field_word, pronoun) = if fields.len() == 1 { ("field", "it") } else { ("fields", "them") };
		let fill = |template: &str| {
			template
				.replace("DATA", &value.serialize())
				.replace("FIELDS", &format!("{field_word} {}", fields.join(", ")))
				.replace("THEM", pronoun)
				.replace("DECLARED", declared)
		};
		(!fields.is_empty()).then(|| Diagnostic::at(value, fill(message)).fix(fill(fix)))
	};
	[
		warning(&missing, MISSING_FIELDS_WARNING, MISSING_FIELDS_FIX),
		warning(&extra, EXTRA_FIELDS_WARNING, EXTRA_FIELDS_FIX),
	]
	.into_iter()
	.flatten()
	.collect()
}

/// `p:photo = …`: the variable and its declared type, when that is a declared type
pub(super) fn typed_target(target: &Node, registry: &TypeRegistry) -> Option<(String, String)> {
	let Node::Key(name, Op::Colon, type_node) = target.drop_meta() else {
		return None;
	};
	let (Node::Symbol(name), Node::Symbol(type_name)) = (name.drop_meta(), type_node.drop_meta()) else {
		return None;
	};
	names_classes(type_name, registry).then(|| (name.clone(), type_name.clone()))
}

/// The types a declared type names: `Circle`, and each part of a union `Circle or Square`, `Point or text` (card union-types)
pub(super) fn class_parts(type_name: &str) -> Vec<&str> {
	crate::analyzer::union_parts(type_name).unwrap_or_else(|| vec![type_name])
}

/// A class, or a union of classes and builtin types with at least one class
pub(super) fn names_classes(type_name: &str, registry: &TypeRegistry) -> bool {
	let parts = class_parts(type_name);
	parts.iter().any(|part| registry.get_by_name(part).is_some())
		&& parts
			.iter()
			.all(|part| registry.get_by_name(part).is_some() || crate::analyzer::builtin_type_kind(part).is_some())
}

/// `image is not a photo`, with the fix `declare image like photo`
pub(super) fn not_like(argument: &Node, needs: &str, given: &str, declared: &str) -> Node {
	let fill = |template: &str| template.replace("GIVEN", given).replace("DECLARED", declared);
	Diagnostic::at(argument, format!("{needs}: {}", fill(NOT_LIKE_MESSAGE))).fix(fill(NOT_LIKE_FIX)).into_error()
}

pub(super) fn declare(node: Node, errors: &mut Vec<Node>) -> Node {
	if let Some(likeness) = like_declaration(&node) {
		return Node::meta(Node::Empty, Node::data(likeness));
	}
	if let Some(declared) = declaration(&node) {
		return match declared {
			Ok(declared) => Node::meta(Node::Empty, Node::data(declared)),
			Err(error) => {
				errors.push(error.clone());
				error
			}
		};
	}
	children_rewritten(node, |child| declare(child, errors))
}
