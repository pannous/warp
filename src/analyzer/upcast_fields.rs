//! P201 (from P203: annotations make code strict, card upcast-field): `s: Shape = Circle("a", 2); s.r` reads a field
//! only a subclass or variant of the declared type has, a compile error hinting at a match or a cast. A name the
//! scope tests (`if s is Circle { s.r }`) is smart-cast and not checked; unannotated variables are read at run time.
//! Checked on the source, before class_methods lowers the classes.

use super::*;
use crate::class_methods::class_members;
use std::collections::HashMap;

/// Each class's parent and own member names
type Classes = HashMap<String, (Option<String>, Vec<String>)>;

pub fn check_upcast_fields(program: &Node) -> Option<Diagnostic> {
	let classes = class_members(program);
	if classes.values().all(|(parent, _)| parent.is_none()) {
		return None;
	}
	check_scope(program, HashMap::new(), &classes)
}

/// The reads in one function body (or the program) of variables declared with a class, then the nested functions
fn check_scope(body: &Node, mut declared: HashMap<String, String>, classes: &Classes) -> Option<Diagnostic> {
	let mut definitions = vec![];
	let mut tested = vec![];
	visit_scope(body, &mut |part| match part {
		Node::Key(target, Op::Assign | Op::Define, _) => {
			if let Some((name, class)) = annotated(target, classes) {
				declared.insert(name, class);
			}
		}
		Node::Key(variable, Op::Eq, class) if is_class(class, classes) => tested.extend(symbol_name(variable)),
		_ => {}
	}, &mut definitions);
	declared.retain(|name, _| !tested.contains(name));
	let mut misread = None;
	visit_scope(body, &mut |part| if misread.is_none() {
		misread = subclass_member_read(part, &declared, classes);
	}, &mut vec![]);
	misread.or_else(|| definitions.into_iter().find_map(|(parameters, body)| {
		let parameters = parameters.iter().filter_map(|parameter| annotated(parameter, classes)).collect();
		check_scope(body, parameters, classes)
	}))
}

/// Every node of the scope, the definitions in it set aside (parameters, body), class bodies skipped
fn visit_scope<'a>(node: &'a Node, action: &mut dyn FnMut(&'a Node), definitions: &mut Vec<(&'a [Node], &'a Node)>) {
	let node = node.drop_meta();
	match node {
		Node::Key(head, Op::Define | Op::Assign, body) if matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_)))) => {
			let Node::List(items, _, _) = head.drop_meta() else { unreachable!("guarded") };
			definitions.push((&items[1..], body));
		}
		Node::Type { .. } => {}
		Node::Key(left, _, right) => {
			action(node);
			visit_scope(left, action, definitions);
			visit_scope(right, action, definitions);
		}
		Node::List(items, _, _) => {
			action(node);
			items.iter().for_each(|item| visit_scope(item, action, definitions));
		}
		_ => action(node),
	}
}

/// `s: Shape` of a declared class: (s, Shape)
fn annotated(target: &Node, classes: &Classes) -> Option<(String, String)> {
	let Node::Key(name, Op::Colon, class) = target.drop_meta() else { return None };
	Some((symbol_name(name)?, symbol_name(class).filter(|class| classes.contains_key(class))?))
}

fn symbol_name(node: &Node) -> Option<String> {
	node.symbol_name().map(String::from)
}

fn is_class(node: &Node, classes: &Classes) -> bool {
	symbol_name(node).is_some_and(|name| classes.contains_key(&name))
}

/// `s.r` or `s.area()` of `s: Shape` where only a descendant of Shape has the member
fn subclass_member_read(node: &Node, declared: &HashMap<String, String>, classes: &Classes) -> Option<Diagnostic> {
	let Node::Key(variable, Op::Dot, member) = node else { return None };
	let variable = symbol_name(variable)?;
	let class = declared.get(&variable)?;
	let member = match member.drop_meta() {
		Node::List(items, Bracket::Round, _) => symbol_name(items.first()?)?,
		other => symbol_name(other)?,
	};
	if chain(class, classes).iter().any(|ancestor| classes[ancestor].1.contains(&member)) {
		return None;
	}
	let owner = classes.keys().filter(|candidate| *candidate != class && chain(candidate, classes).contains(class))
		.filter(|candidate| chain(candidate, classes).iter().any(|ancestor| classes[ancestor].1.contains(&member))).min()?;
	Some(Diagnostic::at(node, format!(
		"{class} has no field {member}, only {owner} has: match {variable} {{ {owner}({member}) => … }} or read ({variable} as {owner}).{member}"
	)))
}

/// The class and its ancestors, as far as they are declared
fn chain(class: &str, classes: &Classes) -> Vec<String> {
	let mut chain = vec![];
	let mut current = Some(class.to_string());
	while let Some(name) = current.filter(|name| classes.contains_key(name) && !chain.contains(name)) {
		current = classes[&name].0.clone();
		chain.push(name);
	}
	chain
}
