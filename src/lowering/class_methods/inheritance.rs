//! `class dog extends animal {…}` (P117): dog takes in the fields and methods of animal and of its mixins (`include
//! Walker`), its own ones override them; `super.speak()` calls the version dog would inherit
use super::*;

/// Every class with the items of the mixins it takes in (`class Duck with Walker {…}`, `include Walker` in its body)
/// after its own, but those it defines itself; the mixin declarations themselves are no classes and go
pub(super) fn with_mixins(node: Node) -> Result<Node, Node> {
	let mut mixins: Vec<(String, Vec<Node>)> = vec![];
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		if name.attribute(crate::warp_parser::MIXIN_WORD).is_some() {
			mixins.push((name.drop_meta().name(), class_items(body)));
		}
	});
	if mixins.is_empty() {
		return Ok(node);
	}
	taking_in_mixins(node, &mixins)
}

pub(super) fn taking_in_mixins(node: Node, mixins: &[(String, Vec<Node>)]) -> Result<Node, Node> {
	match node {
		Node::Type { name, .. } if name.attribute(crate::warp_parser::MIXIN_WORD).is_some() => Ok(Node::Empty),
		Node::Type { name, body } => {
			let mut taken: Vec<String> = name.attribute(crate::warp_parser::WITH_KEYWORD).map(|names| match names.drop_meta() {
				Node::List(names, _, _) => names.iter().map(|name| name.drop_meta().name()).collect(),
				single => vec![single.name()],
			}).unwrap_or_default();
			// `include m` of no declared mixin stays (P139: it loads the module m)
			let is_mixin = |item: &Node| included_mixin(item).is_some_and(|mixin| mixins.iter().any(|(declared, _)| *declared == mixin));
			let (included, own): (Vec<Node>, Vec<Node>) = class_items(&body).into_iter().partition(is_mixin);
			taken.extend(included.iter().filter_map(included_mixin));
			if taken.is_empty() {
				return Ok(Node::Type { name, body });
			}
			let own_names: Vec<String> = own.iter().filter_map(item_name).collect();
			let mut items = own;
			for mixin in taken {
				let Some((_, mixin_items)) = mixins.iter().find(|(declared, _)| *declared == mixin) else {
					return Err(crate::node::error(&format!("{} takes in {mixin}, which is no mixin: declare mixin {mixin}{{…}}", name.drop_meta().name())));
				};
				items.extend(mixin_items.iter().filter(|item| item_name(item).is_none_or(|item| !own_names.contains(&item))).cloned());
			}
			let name = match name.attribute(crate::warp_parser::EXTENDS_KEYWORD) {
				Some(parent) => Node::Symbol(name.drop_meta().name()).with_attribute(crate::warp_parser::EXTENDS_KEYWORD, parent.clone()),
				None => Node::Symbol(name.drop_meta().name()),
			};
			Ok(Node::Type { name: Box::new(name), body: Box::new(Node::List(items, Bracket::Curly, Separator::Semicolon)) })
		}
		Node::List(items, bracket, separator) => {
			let items = items.into_iter().map(|item| taking_in_mixins(item, mixins)).collect::<Result<Vec<_>, _>>()?;
			Ok(Node::List(items.into_iter().filter(|item| !matches!(item, Node::Empty)).collect(), bracket, separator))
		}
		Node::Meta { node, data } => Ok(Node::Meta { node: Box::new(taking_in_mixins(*node, mixins)?), data }),
		other => Ok(other),
	}
}

/// `include Walker` in a class body: the mixin it takes in
pub(super) fn included_mixin(item: &Node) -> Option<String> {
	let Node::List(words, _, _) = item.drop_meta() else { return None };
	let [word, mixin] = words.as_slice() else { return None };
	(word.drop_meta().name() == INCLUDE_WORD && matches!(mixin.drop_meta(), Node::Symbol(_))).then(|| mixin.drop_meta().name())
}

/// The classes with the fields and methods of the class they extend, and the likeness `dog like animal` of each
pub(super) fn inherit(node: Node) -> Result<(Node, Vec<Node>), Node> {
	let mut classes: Vec<(String, Option<String>, Vec<Node>)> = vec![];
	node.visit(&mut |part| {
		if let Node::Type { name, body } = part {
			let parent = name.attribute(crate::warp_parser::EXTENDS_KEYWORD).map(|parent| parent.drop_meta().name());
			classes.push((name.drop_meta().name(), parent, class_items(body)));
		}
	});
	if classes.iter().all(|(_, parent, _)| parent.is_none()) {
		return Ok((node, vec![]));
	}
	let mut likenesses = vec![];
	for (class, parent, _) in &classes {
		if let Some(parent) = parent {
			inherited_items(class, &classes, &mut vec![])?;
			let words = [class.as_str(), crate::traits::LIKE_WORD, parent.as_str()].map(symbol);
			likenesses.push(Node::List(words.to_vec(), Bracket::None, Separator::Space));
		}
	}
	Ok((with_inherited(node, &classes)?, likenesses))
}

/// A class's items after those of its parents: a parent's field or method the class defines again is left out
pub(super) fn inherited_items(class: &str, classes: &[(String, Option<String>, Vec<Node>)], visiting: &mut Vec<String>) -> Result<Vec<Node>, Node> {
	if visiting.iter().any(|seen| seen == class) {
		return Err(crate::node::error(&format!("class {class} extends itself through {}", visiting.join(", "))));
	}
	let Some((_, parent, own)) = classes.iter().find(|(name, _, _)| name == class) else {
		let child = visiting.last().cloned().unwrap_or_default();
		return Err(crate::node::error(&format!("{child} extends {class}, which is no class: declare class {class}{{…}}")));
	};
	let Some(parent) = parent else { return Ok(own.clone()) };
	visiting.push(class.to_string());
	let inherited = inherited_items(parent, classes, visiting)?;
	visiting.pop();
	let mut called = vec![];
	let own: Vec<Node> = own.iter().cloned().map(|item| super_calls(item, class, &mut called)).collect();
	let mut parent_versions = vec![];
	for method in called {
		let Some(version) = inherited.iter().find(|item| method_parts(item).is_some_and(|(name, _, _)| name == method)) else {
			return Err(crate::node::error(&format!("{class} calls {SUPER}.{method}, but {parent} has no method {method}")));
		};
		parent_versions.push(renamed(version, &format!("{method}{SUPER_INFIX}{class}")));
	}
	let own_names: Vec<String> = own.iter().filter_map(item_name).collect();
	let kept = inherited.into_iter().filter(|item| item_name(item).is_none_or(|name| !own_names.contains(&name)));
	Ok(kept.chain(parent_versions).chain(own).collect())
}

/// `super.speak(…)` in a method of class dog as `self.speak·super·dog(…)`; the names of the methods called go to `called`
pub(super) fn super_calls(node: Node, class: &str, called: &mut Vec<String>) -> Node {
	let Node::Key(object, Op::Dot, member) = node else { return node.map_children(|child| super_calls(child, class, called)) };
	if object.drop_meta().name() != SUPER || !matches!(object.drop_meta(), Node::Symbol(_)) {
		return key(super_calls(*object, class, called), Op::Dot, super_calls(*member, class, called));
	}
	let member = match member.drop_meta().clone() {
		Node::Symbol(name) => parent_version(name, class, called),
		Node::List(items, Bracket::Round, separator) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => {
			let arguments: Vec<Node> = items[1..].iter().cloned().map(|argument| super_calls(argument, class, called)).collect();
			let name = parent_version(items[0].drop_meta().name(), class, called);
			Node::List([vec![name], arguments].concat(), Bracket::Round, separator)
		}
		other => other,
	};
	key(symbol(RECEIVER), Op::Dot, member)
}

/// `speak·super·dog`, the name of the speak dog inherits; speak goes to `called`
pub(super) fn parent_version(method: String, class: &str, called: &mut Vec<String>) -> Node {
	let version = format!("{method}{SUPER_INFIX}{class}");
	if !called.contains(&method) {
		called.push(method);
	}
	Node::Symbol(version)
}

/// A method definition under another name
pub(super) fn renamed(method: &Node, name: &str) -> Node {
	let Node::Key(head, Op::Define, body) = method.drop_meta() else { return method.clone() };
	if let Node::Key(untyped, Op::Colon, result_type) = head.drop_meta() {
		let untyped = renamed(&Node::Key(untyped.clone(), Op::Define, body.clone()), name);
		return with_result_type(untyped, Some(result_type));
	}
	let head = match head.drop_meta() {
		Node::List(parts, bracket, separator) => Node::List([vec![symbol(name)], parts[1..].to_vec()].concat(), bracket.clone(), separator.clone()),
		_ => symbol(name),
	};
	Node::Key(Box::new(head), Op::Define, body.clone())
}

/// The name a class item defines: a field or a method
pub(super) fn item_name(item: &Node) -> Option<String> {
	method_parts(item).map(|(name, _, _)| name).or_else(|| field_name(item))
}

/// Every class that extends another with all its items, its name without the parent
pub(super) fn with_inherited(node: Node, classes: &[(String, Option<String>, Vec<Node>)]) -> Result<Node, Node> {
	match node {
		Node::Type { name, body: _ } if name.attribute(crate::warp_parser::EXTENDS_KEYWORD).is_some() => {
			let class = name.drop_meta().name();
			let items = inherited_items(&class, classes, &mut vec![])?;
			Ok(Node::Type { name: Box::new(Node::Symbol(class)), body: Box::new(Node::List(items, Bracket::Curly, Separator::Space)) })
		}
		Node::List(items, bracket, separator) => Ok(Node::List(items.into_iter().map(|item| with_inherited(item, classes)).collect::<Result<_, _>>()?, bracket, separator)),
		Node::Meta { node, data } => Ok(Node::Meta { node: Box::new(with_inherited(*node, classes)?), data }),
		other => Ok(other),
	}
}
