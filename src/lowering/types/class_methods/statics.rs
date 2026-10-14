//! Static members: `static k = 3` and `static make(x) := …` in a class body (P122)
use super::*;
use crate::words::SELF_WORD;

/// A method of the instances of a class, its name and parameters: an instance method, or the getter `k` of a static
/// field `static k = 3`; a static method is none
pub(super) fn instance_member(item: &Node) -> Option<(String, Vec<Node>)> {
	match (is_static(item), method_parts(item)) {
		(false, Some((name, parameters, _))) => Some((name, parameters)),
		(true, None) => declared_field_name(item).map(|name| (name, vec![])),
		_ => None,
	}
}

/// `static k = 3` in a class body (P122)
pub(super) fn is_static(item: &Node) -> bool {
	item.attribute(crate::warp_parser::STATIC_KEYWORD).is_some()
}

/// `circle·count`: the global or function a static member of a class is
pub(super) fn static_name(class: &str, member: &str) -> String {
	format!("{class}·{member}")
}

/// The static members of every class, as (class, member)
pub(super) fn static_members(node: &Node) -> Vec<(String, String)> {
	let mut statics = vec![];
	node.visit(&mut |part| {
		let Node::Type { name, body } = part else { return };
		let class = name.drop_meta().name();
		statics.extend(
			class_items(body)
				.iter()
				.filter(|item| is_static(item))
				.filter_map(item_name)
				.map(|member| (class.clone(), member)),
		);
	});
	statics
}

/// `circle.count` of a static member as the global `circle·count`
pub(super) fn static_reads(node: Node, statics: &[(String, String)]) -> Node {
	let recurse = |child: Node| static_reads(child, statics);
	let Node::Key(object, Op::Dot, member) = node else {
		return node.map_children(recurse);
	};
	let Node::Symbol(class) = object.drop_meta() else {
		return key(recurse(*object), Op::Dot, recurse(*member));
	};
	let is_member = |name: &str| statics.iter().any(|(owner, static_member)| owner == class && static_member == name);
	match member.drop_meta() {
		Node::Symbol(name) if is_member(name) => Node::Symbol(static_name(class, name)),
		_ => Node::Key(object, Op::Dot, Box::new(recurse(*member))),
	}
}

/// `static make(x) := body` as the function `class·make(x) := body`; `static k = 3` as the global `class·k = 3` and the
/// getter `k(self:class) := class·k`, so an instance reads it too (`c.k`)
pub(super) fn static_definitions(members: &Members, member: &Node) -> Vec<Node> {
	let class = members.class;
	let Some(name) = declared_field_name(member) else {
		return vec![];
	};
	let Node::Key(_, Op::Assign, value) = member.drop_meta() else {
		return vec![];
	};
	let global = Node::Symbol(static_name(class, &name));
	let receiver = key(symbol(SELF_WORD), Op::Colon, symbol(class));
	let getter_head = call(&name, vec![receiver]);
	let declaration = Node::Key(Box::new(global.clone()), Op::Assign, value.clone());
	// `global c·n = 0`: methods may change it (`n += 1`)
	let declaration = key(symbol(GLOBAL_WORD), Op::Colon, declaration);
	vec![declaration, key(getter_head, Op::Define, global)]
}
