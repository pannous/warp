//! Methods in a class body (issue #14, notes/classes.md): `class person{name; greet() := "hi " + name}` is the class
//! `person{name}` and the function `greet(self:person) := "hi " + self.name`. In a method body a bare field name and
//! `self`/`this` read the receiver; `p.greet()`, `greet(p)` and, for a method without parameters, `p.area` call it.
//! A method name that several classes define is declared as an implicit trait, so a call picks the class's own method
//! by the static type of its receiver (traits.rs witnesses).

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

/// The receiver parameter of a method
const RECEIVER: &str = "self";
/// Other names of the receiver in a method body
const RECEIVER_ALIASES: [&str; 1] = ["this"];
/// Suffixes of a field name that mark it optional or required (`left?`, `name!`)
const FIELD_MARKS: [char; 2] = ['?', '!'];
/// The trait keyword and the name of the implicit trait of a method several classes define: `has·area`
const TRAIT_KEYWORD: &str = "trait";
const IMPLICIT_TRAIT_PREFIX: &str = "has·";

pub fn lower(node: Node) -> Node {
	let traits = shared_method_traits(&node);
	let shared: Vec<String> = traits.iter().filter_map(trait_operation).collect();
	let mut changing = vec![];
	let node = lower_classes(node, &mut changing);
	// `s.area`, `s.scaled(3)` of a shared method: the call `area(s)` before traits rename each class's method;
	// `c.inc()` of a method changing its object: `c = inc(c)` (P116)
	let node = if shared.is_empty() && changing.is_empty() { node } else { method_calls(node, &shared, &changing) };
	match (traits.is_empty(), node) {
		(true, node) => node,
		(false, Node::List(items, bracket, separator)) if separator != Separator::Space => Node::List([traits, items].concat(), bracket, separator),
		(false, node) => Node::List([traits, vec![node]].concat(), Bracket::None, Separator::Semicolon),
	}
}

/// `trait has·area{area}` for each method name that two or more classes define, taking the first one's parameters
fn shared_method_traits(node: &Node) -> Vec<Node> {
	let mut methods: Vec<(String, Node)> = vec![];
	node.visit(&mut |part| {
		let Node::Type { body, .. } = part else { return };
		let items = match body.drop_meta() {
			Node::List(items, _, _) => items.clone(),
			single => vec![single.clone()],
		};
		for item in items.iter().flat_map(|item| match item.drop_meta() {
			Node::List(group, Bracket::None, _) => group.clone(),
			_ => vec![item.clone()],
		}) {
			if let Some((name, parameters, _)) = method_parts(&item) {
				let receiver = Node::Symbol(RECEIVER.to_string());
				let requirement = match parameters.is_empty() {
					true => Node::Symbol(name.clone()),
					false => Node::List([vec![Node::Symbol(name.clone()), receiver], parameters].concat(), Bracket::Round, Separator::None),
				};
				methods.push((name, requirement));
			}
		}
	});
	let mut traits = vec![];
	for (index, (name, requirement)) in methods.iter().enumerate() {
		let first = methods.iter().position(|(other, _)| other == name) == Some(index);
		let shared = methods.iter().filter(|(other, _)| other == name).count() > 1;
		if first && shared {
			let requirements = Node::List(vec![requirement.clone()], Bracket::Curly, Separator::Space);
			let declaration = Node::Key(Box::new(Node::Symbol(format!("{IMPLICIT_TRAIT_PREFIX}{name}"))), Op::Colon, Box::new(requirements));
			traits.push(Node::List(vec![Node::Symbol(TRAIT_KEYWORD.to_string()), declaration], Bracket::None, Separator::Space));
		}
	}
	traits
}

/// The operation `area` an implicit trait `trait has·area{…}` declares
fn trait_operation(declaration: &Node) -> Option<String> {
	let Node::List(items, _, _) = declaration else { return None };
	Some(items.get(1)?.drop_meta().name().strip_prefix(IMPLICIT_TRAIT_PREFIX)?.to_string())
}

/// `x.m` and `x.m(args)` of the methods `m` as calls `m(x)`, `m(x, args)`; of a method changing its object, on a variable,
/// the update `x = m(x, args)`
fn method_calls(node: Node, called: &[String], changing: &[String]) -> Node {
	let recurse = |child: Node| method_calls(child, called, changing);
	let Node::Key(receiver, Op::Dot, member) = node else { return node.map_children(recurse) };
	let receiver = recurse(*receiver);
	let (name, arguments) = match member.drop_meta() {
		Node::Symbol(name) => (name.clone(), vec![]),
		Node::List(items, Bracket::Round, _) if !items.is_empty() => (items[0].drop_meta().name(), items[1..].iter().cloned().map(recurse).collect()),
		_ => return Node::Key(Box::new(receiver), Op::Dot, Box::new(recurse(*member))),
	};
	if !called.contains(&name) && !changing.contains(&name) {
		return Node::Key(Box::new(receiver), Op::Dot, Box::new(recurse(*member)));
	}
	let call = Node::List([Node::Symbol(name.clone()), receiver.clone()].into_iter().chain(arguments).collect(), Bracket::Round, Separator::None);
	match receiver.drop_meta() {
		Node::Symbol(_) if changing.contains(&name) => Node::Key(Box::new(receiver), Op::Assign, Box::new(call)),
		_ => call,
	}
}

fn lower_classes(node: Node, changing: &mut Vec<String>) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let mut lowered = Vec::with_capacity(items.len());
			for item in items {
				match split_class(&item, changing) {
					Some((class, methods)) => {
						lowered.push(class);
						lowered.extend(methods);
					}
					None => lowered.push(lower_classes(item, changing)),
				}
			}
			Node::List(lowered, bracket, separator)
		}
		other => match split_class(&other, changing) {
			Some((class, methods)) => Node::List([vec![class], methods].concat(), Bracket::None, Separator::Semicolon),
			None => other.map_children(|child| lower_classes(child, changing)),
		},
	}
}

/// A class declaration with methods in its body: the class with its fields only, and the methods as functions; the
/// names of the methods that change their object go to `changing`
fn split_class(node: &Node, changing: &mut Vec<String>) -> Option<(Node, Vec<Node>)> {
	let Node::Type { name, body } = node.drop_meta() else { return None };
	let class = name.drop_meta().name();
	let items = match body.drop_meta() {
		Node::List(items, _, _) => items.clone(),
		single => vec![single.clone()],
	};
	// `{name age:int; greet() := …}`: the fields before a `;` are one group
	let items = items.into_iter().flat_map(|item| match item.drop_meta() {
		Node::List(group, Bracket::None, _) => group.clone(),
		_ => vec![item],
	});
	let (methods, fields): (Vec<Node>, Vec<Node>) = items.partition(|item| method_parts(item).is_some());
	if methods.is_empty() {
		return None;
	}
	let field_names: Vec<String> = fields.iter().filter_map(field_name).collect();
	let functions = methods.iter().map(|method| {
		let (method_name, parameters, method_body) = method_parts(method).expect("partitioned");
		let (function, changes) = function(&class, &field_names, &method_name, parameters, method_body);
		if changes {
			changing.push(method_name);
		}
		function
	}).collect();
	let field_list = Node::List(fields, Bracket::Curly, Separator::Space);
	Some((Node::Type { name: name.clone(), body: Box::new(field_list) }, functions))
}

/// `greet(x) := body` or `area := body` in a class body: name, parameters and body
fn method_parts(item: &Node) -> Option<(String, Vec<Node>, Node)> {
	let Node::Key(head, Op::Define, body) = item.drop_meta() else { return None };
	match head.drop_meta() {
		Node::Symbol(name) => Some((name.clone(), vec![], *body.clone())),
		Node::List(parts, Bracket::Round, _) => match parts.split_first() {
			Some((name, parameters)) if matches!(name.drop_meta(), Node::Symbol(_)) => Some((name.drop_meta().name(), parameters.to_vec(), *body.clone())),
			_ => None,
		},
		_ => None,
	}
}

/// `name`, `age:int`, `left?`: the name of a field
fn field_name(item: &Node) -> Option<String> {
	match item.drop_meta() {
		Node::Symbol(name) => Some(name.trim_end_matches(FIELD_MARKS).to_string()),
		Node::Key(field, Op::Colon, _) => Some(field.drop_meta().name().trim_end_matches(FIELD_MARKS).to_string()),
		_ => None,
	}
}

/// `method(self:class, parameters…) := body`, the fields in the body read from self, and whether it changes a field of
/// self: then it gives the changed object, `(body; self)`
fn function(class: &str, fields: &[String], method: &str, parameters: Vec<Node>, body: Node) -> (Node, bool) {
	let parameter_names: Vec<String> = parameters.iter().map(|parameter| match parameter.drop_meta() {
		Node::Key(name, Op::Colon, _) => name.drop_meta().name(),
		other => other.name(),
	}).collect();
	let readable: Vec<&String> = fields.iter().filter(|field| !parameter_names.contains(field)).collect();
	let body = receiver_reads(body, &readable);
	let changes = changes_receiver(&body);
	let body = match changes {
		true => Node::List(vec![body, Node::Symbol(RECEIVER.to_string())], Bracket::None, Separator::Semicolon),
		false => body,
	};
	let receiver = Node::Key(Box::new(Node::Symbol(RECEIVER.to_string())), Op::Colon, Box::new(Node::Symbol(class.to_string())));
	let head = Node::List([vec![Node::Symbol(method.to_string()), receiver], parameters].concat(), Bracket::Round, Separator::None);
	(Node::Key(Box::new(head), Op::Define, Box::new(body)), changes)
}

/// The body with each field name read from self, `this` as self; the member name right of a dot stays as it is
fn receiver_reads(node: Node, fields: &[&String]) -> Node {
	match node {
		Node::Symbol(name) if fields.contains(&&name) => Node::Key(Box::new(Node::Symbol(RECEIVER.to_string())), Op::Dot, Box::new(Node::Symbol(name))),
		Node::Symbol(name) if RECEIVER_ALIASES.contains(&name.as_str()) => Node::Symbol(RECEIVER.to_string()),
		Node::Key(object, Op::Dot, member) => Node::Key(Box::new(receiver_reads(*object, fields)), Op::Dot, member),
		other => other.map_children(|child| receiver_reads(child, fields)),
	}
}

/// Does the body assign a field of self (`self.n = …`, `n += 1`, `n++`)
fn changes_receiver(body: &Node) -> bool {
	let is_receiver_field = |target: &Node| matches!(target.drop_meta(), Node::Key(object, Op::Dot, _) if object.drop_meta().name() == RECEIVER);
	let mut changes = false;
	body.visit(&mut |part| {
		if let Node::Key(target, op, _) = part {
			changes |= (*op == Op::Assign || op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec)) && is_receiver_field(target);
		}
	});
	changes
}
