//! Methods in a class body (issue #14, notes/classes.md): `class person{name; greet() := "hi " + name}` is the class
//! `person{name}` and the function `greet(self:person) := "hi " + self.name`. In a method body a bare field name and
//! `self`/`this` read the receiver; `p.greet()`, `greet(p)` and, for a method without parameters, `p.area` call it.
//! A method name that several classes define is declared as an implicit trait, so a call picks the class's own method
//! by the static type of its receiver (traits.rs witnesses).
//! `class dog extends animal {…}` (P117) gives dog the fields and methods of animal, its own ones override them, and
//! declares `dog like animal`, so a dog is accepted where an animal is wanted.

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
/// `super.speak()` in a method of dog calls the speak dog would inherit, kept for dog as the method `speak·super·dog`
const SUPER: &str = "super";
const SUPER_INFIX: &str = "·super·";

pub fn lower(node: Node) -> Node {
	let (node, likenesses) = match inherit(node) {
		Ok(inherited) => inherited,
		Err(error) => return error,
	};
	let traits = [shared_method_traits(&node), likenesses].concat();
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

/// The classes with the fields and methods of the class they extend, and the likeness `dog like animal` of each
fn inherit(node: Node) -> Result<(Node, Vec<Node>), Node> {
	let mut classes: Vec<(String, Option<String>, Vec<Node>)> = vec![];
	node.visit(&mut |part| {
		if let Node::Type { name, body } = part {
			let parent = name.attribute(crate::wasp_parser::EXTENDS_KEYWORD).map(|parent| parent.drop_meta().name());
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
			let words = [class.as_str(), crate::traits::LIKE_WORD, parent.as_str()].map(|word| Node::Symbol(word.to_string()));
			likenesses.push(Node::List(words.to_vec(), Bracket::None, Separator::Space));
		}
	}
	Ok((with_inherited(node, &classes)?, likenesses))
}

/// The fields and methods of a class body; the groups a `;` makes (`{name age:int; greet() := …}`) flattened
fn class_items(body: &Node) -> Vec<Node> {
	let items = match body.drop_meta() {
		Node::List(items, _, _) => items.clone(),
		Node::Empty => vec![],
		single => vec![single.clone()],
	};
	items.into_iter().flat_map(|item| match item.drop_meta() {
		Node::List(group, Bracket::None, _) => group.clone(),
		_ => vec![item],
	}).collect()
}

/// A class's items after those of its parents: a parent's field or method the class defines again is left out
fn inherited_items(class: &str, classes: &[(String, Option<String>, Vec<Node>)], visiting: &mut Vec<String>) -> Result<Vec<Node>, Node> {
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
fn super_calls(node: Node, class: &str, called: &mut Vec<String>) -> Node {
	let Node::Key(object, Op::Dot, member) = node else { return node.map_children(|child| super_calls(child, class, called)) };
	if object.drop_meta().name() != SUPER || !matches!(object.drop_meta(), Node::Symbol(_)) {
		return Node::Key(Box::new(super_calls(*object, class, called)), Op::Dot, Box::new(super_calls(*member, class, called)));
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
	Node::Key(Box::new(Node::Symbol(RECEIVER.to_string())), Op::Dot, Box::new(member))
}

/// `speak·super·dog`, the name of the speak dog inherits; speak goes to `called`
fn parent_version(method: String, class: &str, called: &mut Vec<String>) -> Node {
	let version = format!("{method}{SUPER_INFIX}{class}");
	if !called.contains(&method) {
		called.push(method);
	}
	Node::Symbol(version)
}

/// A method definition under another name
fn renamed(method: &Node, name: &str) -> Node {
	let Node::Key(head, Op::Define, body) = method.drop_meta() else { return method.clone() };
	let head = match head.drop_meta() {
		Node::List(parts, bracket, separator) => Node::List([vec![Node::Symbol(name.to_string())], parts[1..].to_vec()].concat(), bracket.clone(), separator.clone()),
		_ => Node::Symbol(name.to_string()),
	};
	Node::Key(Box::new(head), Op::Define, body.clone())
}

/// The name a class item defines: a field or a method
fn item_name(item: &Node) -> Option<String> {
	method_parts(item).map(|(name, _, _)| name).or_else(|| field_name(item))
}

/// Every class that extends another with all its items, its name without the parent
fn with_inherited(node: Node, classes: &[(String, Option<String>, Vec<Node>)]) -> Result<Node, Node> {
	match node {
		Node::Type { name, body: _ } if name.attribute(crate::wasp_parser::EXTENDS_KEYWORD).is_some() => {
			let class = name.drop_meta().name();
			let items = inherited_items(&class, classes, &mut vec![])?;
			Ok(Node::Type { name: Box::new(Node::Symbol(class)), body: Box::new(Node::List(items, Bracket::Curly, Separator::Space)) })
		}
		Node::List(items, bracket, separator) => Ok(Node::List(items.into_iter().map(|item| with_inherited(item, classes)).collect::<Result<_, _>>()?, bracket, separator)),
		Node::Meta { node, data } => Ok(Node::Meta { node: Box::new(with_inherited(*node, classes)?), data }),
		other => Ok(other),
	}
}

/// `trait has·area{area}` for each method name that two or more classes define, taking the first one's parameters
fn shared_method_traits(node: &Node) -> Vec<Node> {
	let mut methods: Vec<(String, Node)> = vec![];
	node.visit(&mut |part| {
		let Node::Type { body, .. } = part else { return };
		for item in class_items(body) {
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
	let (methods, fields): (Vec<Node>, Vec<Node>) = class_items(body).into_iter().partition(|item| method_parts(item).is_some());
	if methods.is_empty() {
		return None;
	}
	let field_names: Vec<String> = fields.iter().filter_map(field_name).collect();
	let siblings: Vec<(String, bool)> = methods.iter().filter_map(method_parts).map(|(name, parameters, _)| (name, parameters.is_empty())).collect();
	let functions = methods.iter().map(|method| {
		let (method_name, parameters, method_body) = method_parts(method).expect("partitioned");
		let members = Members { fields: &field_names, methods: &siblings };
		let (function, changes) = function(&class, &members, &method_name, parameters, method_body);
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

/// The fields of a class and its methods, each with whether it takes no parameters (a getter)
struct Members<'a> {
	fields: &'a [String],
	methods: &'a [(String, bool)],
}

/// `method(self:class, parameters…) := body`, the fields and methods in the body read from self, and whether it changes
/// a field of self: then it gives the changed object, `(body; self)`
fn function(class: &str, members: &Members, method: &str, parameters: Vec<Node>, body: Node) -> (Node, bool) {
	let parameter_names: Vec<String> = parameters.iter().map(|parameter| match parameter.drop_meta() {
		Node::Key(name, Op::Colon, _) => name.drop_meta().name(),
		other => other.name(),
	}).collect();
	let unshadowed = |name: &String| !parameter_names.contains(name);
	let fields: Vec<&String> = members.fields.iter().filter(|field| unshadowed(field)).collect();
	let methods: Vec<&String> = members.methods.iter().map(|(name, _)| name).filter(|name| unshadowed(name)).collect();
	let getters: Vec<&String> = members.methods.iter().filter(|(_, getter)| *getter).map(|(name, _)| name).filter(|name| unshadowed(name)).collect();
	let readable = Readable { fields: [fields, getters].concat(), methods };
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

/// The names a method body reads from its receiver: fields and getters (`area`), and methods it calls (`area()`)
struct Readable<'a> {
	fields: Vec<&'a String>,
	methods: Vec<&'a String>,
}

/// The body with each field name and getter read from self, each call of a method of the class a call on self (as in
/// Java), `this` as self; the member name right of a dot stays as it is
fn receiver_reads(node: Node, readable: &Readable) -> Node {
	let on_receiver = |member: Node| Node::Key(Box::new(Node::Symbol(RECEIVER.to_string())), Op::Dot, Box::new(member));
	match node {
		Node::Symbol(name) if readable.fields.contains(&&name) => on_receiver(Node::Symbol(name)),
		Node::Symbol(name) if RECEIVER_ALIASES.contains(&name.as_str()) => Node::Symbol(RECEIVER.to_string()),
		Node::List(items, Bracket::Round, separator) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if readable.methods.contains(&name)) => {
			let arguments = items[1..].iter().cloned().map(|argument| receiver_reads(argument, readable));
			on_receiver(Node::List(std::iter::once(items[0].clone()).chain(arguments).collect(), Bracket::Round, separator))
		}
		Node::Key(object, Op::Dot, member) => Node::Key(Box::new(receiver_reads(*object, readable)), Op::Dot, member),
		other => other.map_children(|child| receiver_reads(child, readable)),
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
