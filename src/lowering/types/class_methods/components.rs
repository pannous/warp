//! Class components (notes/web_framework.md step 4): an instance of a class with a render method is rendered, `view()` is its render
use super::*;

/// Class components (notes/web_framework.md step 4): an instance of a class with a render() method is its markup where
/// markup is expected, a child of an element (`div{ Greeting("Ann") }`) or the program's value; anywhere else it stays
/// an instance
pub(super) fn rendered_components(node: Node) -> Node {
	let node = with_render_aliases(node);
	let mut components = vec![];
	node.visit(&mut |part| {
		if let Node::Type { name, body } = part {
			if class_items(body).iter().filter_map(method_parts).any(|(method, _, _)| method == RENDER_WORD) {
				components.push(name.drop_meta().name());
			}
		}
	});
	if components.is_empty() {
		return node;
	}
	let node = with_rendered_children(node, &components);
	match node {
		Node::List(mut statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
			if let Some(last) = statements.pop() {
				statements.push(rendered(last, &components));
			}
			Node::List(statements, Bracket::None, separator)
		}
		single => rendered(single, &components),
	}
}

/// `view() := …` in a class without its own render: `render() := …`
pub(super) fn with_render_aliases(node: Node) -> Node {
	match node {
		Node::Type { name, body } => {
			let items = class_items(&body);
			let methods: Vec<String> = items.iter().filter_map(method_parts).map(|(method, _, _)| method).collect();
			let alias = RENDER_ALIASES.iter().find(|alias| methods.iter().any(|method| method == *alias));
			match alias.filter(|_| !methods.iter().any(|method| method == RENDER_WORD)) {
				Some(alias) => {
					crate::errors::diagnostic::note_alias(alias, RENDER_WORD);
					let items = items
						.into_iter()
						.map(|item| match method_parts(&item) {
							Some((method, _, _)) if method == *alias => renamed(&item, RENDER_WORD),
							_ => item,
						})
						.collect();
					Node::Type { name, body: Box::new(Node::List(items, Bracket::Curly, Separator::Semicolon)) }
				}
				None => Node::Type { name, body },
			}
		}
		other => other.map_children(with_render_aliases),
	}
}

/// The children of each element rendered
pub(super) fn with_rendered_children(node: Node, components: &[String]) -> Node {
	match node {
		Node::Key(tag, op @ (Op::None | Op::Colon), children)
			if crate::web::markup::is_element_tag(&tag.drop_meta().name())
				&& matches!(children.drop_meta(), Node::List(_, Bracket::Curly, _)) =>
		{
			let Node::List(items, bracket, separator) = children.drop_meta().clone() else { unreachable!("guarded") };
			let items =
				items.into_iter().map(|item| rendered(with_rendered_children(item, components), components)).collect();
			Node::Key(tag, op, Box::new(Node::List(items, bracket, separator)))
		}
		other => other.map_children(|child| with_rendered_children(child, components)),
	}
}

/// A construction of a component as `Greeting("Ann").render()`
pub(super) fn rendered(node: Node, components: &[String]) -> Node {
	let is_construction = matches!(node.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().is_some_and(|class| components.contains(&class.drop_meta().name())));
	match is_construction {
		true => key(node, Op::Dot, call(RENDER_WORD, vec![])),
		false => node,
	}
}

/// `Point.from_json(t)` reads the json, as `parse_json(t) as Point` (std json)
pub(super) const FROM_JSON_WORD: &str = "from_json";
pub(super) const PARSE_JSON_WORD: &str = "parse_json";
/// The variable holding the object an instance is built from: `Point·from`
pub(super) const FROM_SUFFIX: &str = "·from";
/// The element of a list of instances built from objects: `Point·element`
pub(super) const ELEMENT_SUFFIX: &str = "·element";
/// The variable of a list of instances built before its construction: `elements·1`
pub(super) const ELEMENTS_WORD: &str = "elements";
