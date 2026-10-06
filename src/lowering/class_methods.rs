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
const GLOBAL_KEYWORD: &str = "global";
/// The constructor block of a class body, `value{ id = random() }` or `value(name){…}` (wiki/constructor.md)
const VALUE_WORD: &str = crate::wasp_parser::CONSTRUCTOR_WORD;
/// The suffix of a method named like a type word: `double·method`
const METHOD_SUFFIX: &str = "·method";

/// The function a class's `value{…}` block becomes, `person·value(self:person)`: every construction is passed through it
pub fn constructor_name(class: &str) -> String {
	format!("{class}·{VALUE_WORD}")
}

pub fn lower(node: Node) -> Node {
	let node = renamed_type_word_methods(node);
	let (node, likenesses) = match inherit(node) {
		Ok(inherited) => inherited,
		Err(error) => return error,
	};
	let traits = [shared_method_traits(&node), likenesses].concat();
	let shared: Vec<String> = traits.iter().filter_map(trait_operation).collect();
	let mut changing = vec![];
	let statics = static_members(&node);
	let node = lower_classes(node, &mut changing);
	let node = if statics.is_empty() { node } else { static_reads(node, &statics) };
	// `s.area`, `s.scaled(3)` of a shared method: the call `area(s)` before traits rename each class's method;
	// `c.inc()` of a method changing its object: `c = inc(c)` (P116)
	let node = if shared.is_empty() && changing.is_empty() { node } else { method_calls(node, &shared, &changing) };
	match (traits.is_empty(), node) {
		(true, node) => node,
		(false, Node::List(items, bracket, separator)) if separator != Separator::Space => Node::List([traits, items].concat(), bracket, separator),
		(false, node) => Node::List([traits, vec![node]].concat(), Bracket::None, Separator::Semicolon),
	}
}

/// A method named like a type word (`double() := x*2`, P142: class methods are always allowed) under the name
/// `double·method`, so its calls `c.double()` (and `double()` in the class body) never read as the cast `double(c)`
fn renamed_type_word_methods(node: Node) -> Node {
	let mut names = vec![];
	node.visit(&mut |part| if let Node::Type { body, .. } = part {
		names.extend(class_items(body).iter().filter_map(method_parts).map(|(name, _, _)| name).filter(|name| crate::analyzer::type_word_kind(name).is_some()));
	});
	if names.is_empty() { node } else { with_method_names(node, &names, false) }
}

fn method_name(name: &str) -> String {
	format!("{name}{METHOD_SUFFIX}")
}

/// The type-word methods `names` renamed where they are defined and called: `c.double`, `c.double()`, and in a class
/// body `double()` and the definition `double() := …`
fn with_method_names(node: Node, names: &[String], in_class: bool) -> Node {
	let renamed = |name: &Node| match name.drop_meta() {
		Node::Symbol(word) if names.contains(word) => Some(Node::Symbol(method_name(word))),
		_ => None,
	};
	let renamed_call = |call: &Node| match call.drop_meta() {
		Node::List(items, Bracket::Round, separator) if !items.is_empty() => {
			renamed(&items[0]).map(|name| Node::List([vec![name], items[1..].to_vec()].concat(), Bracket::Round, separator.clone()))
		}
		_ => None,
	};
	let recurse = |child: Node| with_method_names(child, names, in_class);
	match node {
		Node::Type { name, body } => Node::Type { name, body: Box::new(with_method_names(*body, names, true)) },
		// `3.double()` of a number stays the conversion
		Node::Key(receiver, Op::Dot, member) if !matches!(receiver.drop_meta(), Node::Number(_)) => {
			let member = renamed(&member).or_else(|| renamed_call(&member)).unwrap_or_else(|| recurse(*member));
			Node::Key(Box::new(recurse(*receiver)), Op::Dot, Box::new(member))
		}
		Node::List(..) if in_class && renamed_call(&node).is_some() => {
			let Node::List(items, bracket, separator) = renamed_call(&node).expect("guarded") else { unreachable!("a call") };
			Node::List(items.into_iter().map(recurse).collect(), bracket, separator)
		}
		// `double := …`, a getter
		Node::Key(head, Op::Define, body) if in_class && renamed(&head).is_some() => Node::Key(Box::new(renamed(&head).expect("guarded")), Op::Define, Box::new(recurse(*body))),
		other => other.map_children(recurse),
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
		Node::List(words, _, _) if keyword_method(words).is_some() => keyword_method(words).into_iter().collect(),
		Node::List(words, _, _) if value_block(words).is_some() => value_block(words).into_iter().collect(),
		Node::List(group, Bracket::None, _) => group.clone(),
		_ => vec![item],
	}).collect()
}

/// `def area() -> int {…}`, `fun area(): Int {…}`, `func area() {…}`: the method `area() := …` (with its result type)
fn keyword_method(words: &[Node]) -> Option<Node> {
	crate::declarations::keyword_definition(words)
}

/// `value {…}` (spaced) and `value(name) {…}`: the constructor `value{…}`, `value(name):{…}`
fn value_block(words: &[Node]) -> Option<Node> {
	let [word, block] = words else { return None };
	let is_block = matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _));
	(is_block && constructor_parameters(word).is_some()).then(|| Node::Key(Box::new(word.clone()), Op::None, Box::new(block.clone())))
}

/// The parameters `value` or `value(name)` declares
fn constructor_parameters(word: &Node) -> Option<Vec<Node>> {
	match word.drop_meta() {
		Node::Symbol(name) if name == VALUE_WORD => Some(vec![]),
		// `value(a, b)`, `value (n)`: the parameters may come as one group
		Node::List(call, Bracket::Round, _) if matches!(call.first().map(Node::drop_meta), Some(Node::Symbol(name)) if name == VALUE_WORD) => {
			Some(call[1..].iter().flat_map(|parameter| match parameter.drop_meta() {
				Node::List(group, Bracket::Round, _) => group.clone(),
				_ => vec![parameter.clone()],
			}).collect())
		}
		_ => None,
	}
}

/// The parameters and statements of the constructor `value{…}`, `value(name){…}`
fn constructor_parts(item: &Node) -> Option<(Vec<Node>, &Node)> {
	match item.drop_meta() {
		// glued `value{…}` arrives as the key `value:{…}`
		Node::Key(word, Op::None | Op::Colon, block) if matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _)) => Some((constructor_parameters(word)?, block)),
		_ => None,
	}
}

fn constructor_body(item: &Node) -> Option<&Node> {
	constructor_parts(item).map(|(_, body)| body)
}

/// The fields a constructor body sets, `id = …` or `this.id = …`
fn fields_set(body: &Node) -> Vec<String> {
	let mut names = vec![];
	body.visit(&mut |part| if let Node::Key(target, Op::Assign, _) = part {
		let name = match target.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			Node::Key(receiver, Op::Dot, field) if RECEIVER_ALIASES.contains(&receiver.drop_meta().name().as_str()) || receiver.drop_meta().name() == RECEIVER => Some(field.drop_meta().name()),
			_ => None,
		};
		names.extend(name.filter(|name| !names.contains(name)));
	});
	names
}

/// The result type a method declares: `area():int := …`
fn result_type(item: &Node) -> Option<&Node> {
	match item.drop_meta() {
		Node::Key(head, Op::Define, _) => match head.drop_meta() {
			Node::Key(_, Op::Colon, result_type) => Some(result_type),
			_ => None,
		},
		_ => None,
	}
}

/// A definition with the result type on its head
fn with_result_type(definition: Node, result_type: Option<&Node>) -> Node {
	match (definition, result_type) {
		(Node::Key(head, Op::Define, body), Some(result_type)) => Node::Key(Box::new(Node::Key(head, Op::Colon, Box::new(result_type.clone()))), Op::Define, body),
		(definition, _) => definition,
	}
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
	if let Node::Key(untyped, Op::Colon, result_type) = head.drop_meta() {
		let untyped = renamed(&Node::Key(untyped.clone(), Op::Define, body.clone()), name);
		return with_result_type(untyped, Some(result_type));
	}
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
			if let Some((name, parameters)) = instance_member(&item) {
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

/// A method of the instances of a class, its name and parameters: an instance method, or the getter `k` of a static
/// field `static k = 3`; a static method is none
fn instance_member(item: &Node) -> Option<(String, Vec<Node>)> {
	match (is_static(item), method_parts(item)) {
		(false, Some((name, parameters, _))) => Some((name, parameters)),
		(true, None) => field_name(item).map(|name| (name, vec![])),
		_ => None,
	}
}

/// `static k = 3`, `static make(x) := …` in a class body (P122)
fn is_static(item: &Node) -> bool {
	item.attribute(crate::wasp_parser::STATIC_KEYWORD).is_some()
}

/// `circle·count`: the global or function a static member of a class is
fn static_name(class: &str, member: &str) -> String {
	format!("{class}·{member}")
}

/// The static members of every class, as (class, member)
fn static_members(node: &Node) -> Vec<(String, String)> {
	let mut statics = vec![];
	node.visit(&mut |part| {
		let Node::Type { name, body } = part else { return };
		let class = name.drop_meta().name();
		statics.extend(class_items(body).iter().filter(|item| is_static(item)).filter_map(item_name).map(|member| (class.clone(), member)));
	});
	statics
}

/// `circle.count` and `circle.make(2)` of static members as the global `circle·count` and the call `circle·make(2)`
fn static_reads(node: Node, statics: &[(String, String)]) -> Node {
	let recurse = |child: Node| static_reads(child, statics);
	let Node::Key(object, Op::Dot, member) = node else { return node.map_children(recurse) };
	let Node::Symbol(class) = object.drop_meta() else { return Node::Key(Box::new(recurse(*object)), Op::Dot, Box::new(recurse(*member))) };
	let is_member = |name: &str| statics.iter().any(|(owner, static_member)| owner == class && static_member == name);
	match member.drop_meta() {
		Node::Symbol(name) if is_member(name) => Node::Symbol(static_name(class, name)),
		Node::List(items, Bracket::Round, separator) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if is_member(name)) => {
			let arguments = items[1..].iter().cloned().map(recurse);
			let function = Node::Symbol(static_name(class, &items[0].drop_meta().name()));
			Node::List(std::iter::once(function).chain(arguments).collect(), Bracket::Round, separator.clone())
		}
		_ => Node::Key(object, Op::Dot, Box::new(recurse(*member))),
	}
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
	let (statics, items): (Vec<Node>, Vec<Node>) = class_items(body).into_iter().partition(is_static);
	let (constructors, items): (Vec<Node>, Vec<Node>) = items.into_iter().partition(|item| constructor_body(item).is_some());
	let (methods, mut fields): (Vec<Node>, Vec<Node>) = items.into_iter().partition(|item| method_parts(item).is_some());
	if methods.is_empty() && statics.is_empty() && constructors.is_empty() {
		return None;
	}
	let mut field_names: Vec<String> = fields.iter().filter_map(field_name).collect();
	// a field only the constructor sets is optional: ø until it runs
	for (parameters, body) in constructors.iter().filter_map(constructor_parts) {
		let parameter_names: Vec<String> = parameters.iter().map(|parameter| parameter.drop_meta().name()).collect();
		for added in fields_set(body).into_iter().filter(|name| !parameter_names.contains(name)) {
			if !field_names.contains(&added) {
				fields.push(Node::Symbol(format!("{added}{}", FIELD_MARKS[0])));
				field_names.push(added);
			}
		}
	}
	let static_names: Vec<String> = statics.iter().filter_map(item_name).collect();
	let siblings: Vec<(String, bool)> = methods.iter().filter_map(method_parts).map(|(name, parameters, _)| (name, parameters.is_empty())).collect();
	let members = Members { class: &class, fields: &field_names, methods: &siblings, statics: &static_names };
	let mut functions: Vec<Node> = statics.iter().flat_map(|member| static_definitions(&members, member)).collect();
	// the constructor gives the instance it was handed, its fields set
	let gives_itself = |body: &Node| Node::List(vec![body.clone(), Node::Symbol(RECEIVER.to_string())], Bracket::None, Separator::Semicolon);
	functions.extend(constructors.iter().filter_map(constructor_parts).map(|(parameters, body)| function(&members, &constructor_name(&class), parameters, gives_itself(body)).0));
	functions.extend(methods.iter().map(|method| {
		let (method_name, parameters, method_body) = method_parts(method).expect("partitioned");
		let (function, changes) = function(&members, &method_name, parameters, method_body);
		if changes {
			changing.push(method_name);
			return function; // it gives its changed object
		}
		with_result_type(function, result_type(method))
	}));
	let field_list = Node::List(fields, Bracket::Curly, Separator::Space);
	Some((Node::Type { name: name.clone(), body: Box::new(field_list) }, functions))
}

/// `static make(x) := body` as the function `class·make(x) := body`; `static k = 3` as the global `class·k = 3` and the
/// getter `k(self:class) := class·k`, so an instance reads it too (`c.k`)
fn static_definitions(members: &Members, member: &Node) -> Vec<Node> {
	let class = members.class;
	if let Some((name, parameters, body)) = method_parts(member) {
		let readable = Readable { class, fields: vec![], methods: vec![], statics: members.statics.iter().collect() };
		let head = Node::List([vec![Node::Symbol(static_name(class, &name))], parameters].concat(), Bracket::Round, Separator::None);
		return vec![Node::Key(Box::new(head), Op::Define, Box::new(receiver_reads(body, &readable)))];
	}
	let Some(name) = field_name(member) else { return vec![] };
	let Node::Key(_, Op::Assign, value) = member.drop_meta() else { return vec![] };
	let global = Node::Symbol(static_name(class, &name));
	let receiver = Node::Key(Box::new(Node::Symbol(RECEIVER.to_string())), Op::Colon, Box::new(Node::Symbol(class.to_string())));
	let getter_head = Node::List(vec![Node::Symbol(name), receiver], Bracket::Round, Separator::None);
	let declaration = Node::Key(Box::new(global.clone()), Op::Assign, value.clone());
	// `global c·n = 0`: methods may change it (`n += 1`)
	let declaration = Node::Key(Box::new(Node::Symbol(GLOBAL_KEYWORD.to_string())), Op::Colon, Box::new(declaration));
	vec![declaration, Node::Key(Box::new(getter_head), Op::Define, Box::new(global))]
}

/// `greet(x) := body` or `area := body` in a class body: name, parameters and body
fn method_parts(item: &Node) -> Option<(String, Vec<Node>, Node)> {
	let Node::Key(head, Op::Define, body) = item.drop_meta() else { return None };
	let head = match head.drop_meta() {
		Node::Key(untyped, Op::Colon, _) if result_type(item).is_some() => untyped,
		_ => head,
	};
	match head.drop_meta() {
		Node::Symbol(name) => Some((name.clone(), vec![], *body.clone())),
		Node::List(parts, Bracket::Round, _) => match parts.split_first() {
			Some((name, parameters)) if matches!(name.drop_meta(), Node::Symbol(_)) => Some((name.drop_meta().name(), parameters.to_vec(), *body.clone())),
			_ => None,
		},
		_ => None,
	}
}

/// `name`, `age:int`, `left?`, `x:int=0`, `k = 3`: the name of a field
fn field_name(item: &Node) -> Option<String> {
	match item.drop_meta() {
		Node::Symbol(name) => Some(name.trim_end_matches(FIELD_MARKS).to_string()),
		Node::Key(field, Op::Colon, _) => Some(field.drop_meta().name().trim_end_matches(FIELD_MARKS).to_string()),
		Node::Key(field, Op::Assign, _) => field_name(field),
		_ => None,
	}
}

/// The fields of a class and its methods, each with whether it takes no parameters (a getter)
struct Members<'a> {
	class: &'a str,
	fields: &'a [String],
	methods: &'a [(String, bool)],
	statics: &'a [String],
}

/// `method(self:class, parameters…) := body`, the fields and methods in the body read from self, and whether it changes
/// a field of self: then it gives the changed object, `(body; self)`
fn function(members: &Members, method: &str, parameters: Vec<Node>, body: Node) -> (Node, bool) {
	let class = members.class;
	let parameter_names: Vec<String> = parameters.iter().map(|parameter| match parameter.drop_meta() {
		Node::Key(name, Op::Colon, _) => name.drop_meta().name(),
		other => other.name(),
	}).collect();
	let unshadowed = |name: &String| !parameter_names.contains(name);
	let fields: Vec<&String> = members.fields.iter().filter(|field| unshadowed(field)).collect();
	let methods: Vec<&String> = members.methods.iter().map(|(name, _)| name).filter(|name| unshadowed(name)).collect();
	let getters: Vec<&String> = members.methods.iter().filter(|(_, getter)| *getter).map(|(name, _)| name).filter(|name| unshadowed(name)).collect();
	let statics: Vec<&String> = members.statics.iter().filter(|name| unshadowed(name)).collect();
	let readable = Readable { class, fields: [fields, getters].concat(), methods, statics };
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
	class: &'a str,
	fields: Vec<&'a String>,
	methods: Vec<&'a String>,
	statics: Vec<&'a String>,
}

/// The body with each field name and getter read from self, each call of a method of the class a call on self (as in
/// Java), `this` as self; the member name right of a dot stays as it is
fn receiver_reads(node: Node, readable: &Readable) -> Node {
	let on_receiver = |member: Node| Node::Key(Box::new(Node::Symbol(RECEIVER.to_string())), Op::Dot, Box::new(member));
	let is_call_of = |items: &[Node], names: &[&String]| matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if names.contains(&name));
	match node {
		Node::Symbol(name) if readable.statics.contains(&&name) => Node::Symbol(static_name(readable.class, &name)),
		Node::List(items, Bracket::Round, separator) if is_call_of(&items, &readable.statics) => {
			let arguments = items[1..].iter().cloned().map(|argument| receiver_reads(argument, readable));
			let function = Node::Symbol(static_name(readable.class, &items[0].drop_meta().name()));
			Node::List(std::iter::once(function).chain(arguments).collect(), Bracket::Round, separator)
		}
		Node::Symbol(name) if readable.fields.contains(&&name) => on_receiver(Node::Symbol(name)),
		Node::Symbol(name) if RECEIVER_ALIASES.contains(&name.as_str()) => Node::Symbol(RECEIVER.to_string()),
		Node::List(items, Bracket::Round, separator) if is_call_of(&items, &readable.methods) => {
			let arguments = items[1..].iter().cloned().map(|argument| receiver_reads(argument, readable));
			on_receiver(Node::List(std::iter::once(items[0].clone()).chain(arguments).collect(), Bracket::Round, separator))
		}
		Node::Key(object, Op::Dot, member) => Node::Key(Box::new(receiver_reads(*object, readable)), Op::Dot, member),
		other => other.map_children(|child| receiver_reads(child, readable)),
	}
}

/// Does the body assign a field of self (`self.n = …`, `n += 1`, `n++`) or change a list in one (`items.add(x)`)
fn changes_receiver(body: &Node) -> bool {
	let mut changes = false;
	body.visit(&mut |part| {
		changes |= match part {
			Node::Key(target, Op::Dot, call) if is_receiver_field(target) => mutating_call(call),
			Node::Key(target, op, _) => (*op == Op::Assign || op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec)) && is_receiver_field(target),
			_ => false,
		};
	});
	changes
}

/// `self.items`, `self.stack.items`
fn is_receiver_field(target: &Node) -> bool {
	match target.drop_meta() {
		Node::Key(object, Op::Dot, _) => matches!(object.drop_meta(), Node::Symbol(name) if name == RECEIVER) || is_receiver_field(object),
		_ => false,
	}
}

/// `add(x)`, `insert(x, at:1)`: a call of a method that changes the list it is called on
fn mutating_call(call: &Node) -> bool {
	matches!(call.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if crate::analyzer::is_list_mutating_method(name)))
}
