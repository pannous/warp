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
use crate::wasp_parser::CONSTRUCTOR_WORD;
/// A property's setter `set age(v) {…}` is the method `age·set(self, v)`, run by `p.age = v`
const SETTER_SUFFIX: &str = "·set";
/// The value a method changing its object gives besides it, `pop·value` in the method and `pop·result` of a call, and
/// the list mutations that give one
const VALUE_SUFFIX: &str = "·value";
const RESULT_SUFFIX: &str = "·result";
const GIVING_MUTATIONS: [&str; 2] = ["pop", "remove"];
/// The keywords of a field: Swift's `var count = 0`, `let`, Kotlin's `val`
const FIELD_KEYWORDS: [&str; 3] = ["var", "let", "val"];
/// Member modifiers that may mean something in wasp, so they get no note that wasp needs them not
const SILENT_MODIFIERS: [&str; 1] = ["async"];
/// The constructor names of other languages, aliases of `init` (P162): wasp's old `value`, JavaScript, Python, Ruby,
/// PHP, VB.NET, Delphi, Rust; besides a method named like its class (C++, Java, C#)
const CONSTRUCTOR_ALIASES: [&str; 8] = ["value", "constructor", "__init__", "initialize", "__construct", "New", "Create", "new"];
/// Rust's block of methods of a type, `impl Point {…}`
const IMPL_WORD: &str = "impl";
/// Kotlin's `p.copy(y = 5)`
const COPY_WORD: &str = "copy";
/// The methods an operator on an instance calls (wiki/operator.md aliases, Python's special methods)
const OPERATOR_METHODS: [(Op, [&str; 3]); 8] = [
	(Op::Add, ["plus", "add", "__add__"]),
	(Op::Sub, ["minus", "subtract", "__sub__"]),
	(Op::Mul, ["times", "multiply", "__mul__"]),
	(Op::Div, ["divide", "div", "__truediv__"]),
	(Op::Mod, ["mod", "modulo", "__mod__"]),
	(Op::Lt, ["less", "smaller", "__lt__"]),
	(Op::Gt, ["more", "bigger", "__gt__"]),
	(Op::Eq, ["equals", "equal", "__eq__"]),
];
/// Ruby's `include Walker` in a class body takes in a mixin
const INCLUDE_WORD: &str = "include";
/// The prefix of a method named like a type or library word: `method·double`, `method·sum` (a prefix: a name
/// `sum·T` reads as the witness of sum for T and would shadow the library's sum)
const METHOD_PREFIX: &str = "method·";

/// The method an operator on an instance calls, `plus` for `+`
pub(crate) fn operator_method(op: Op) -> Option<&'static str> {
	OPERATOR_METHODS.iter().find(|(known, _)| *known == op).map(|(_, names)| names[0])
}

/// A method named by an operator's glyph, `+(o) := …`: the operator's method `plus`
fn glyph_method(name: &str) -> Option<&'static str> {
	OPERATOR_METHODS.iter().find(|(op, _)| op.as_str() == name).map(|(_, names)| names[0])
}

/// The function a class's `init{…}` block becomes, `person·init(self:person)`: every construction is passed through it
pub fn constructor_name(class: &str) -> String {
	format!("{class}·{CONSTRUCTOR_WORD}")
}

pub fn lower(node: Node) -> Node {
	let node = copies(with_impls(node));
	let node = with_init_constructors(node.clone(), &method_calls_named(&node));
	let node = with_members(node);
	let node = with_class_attributes(class_typed_declarations(node));
	let node = operator_calls(node);
	let node = match with_mixins(node) {
		Ok(node) => node,
		Err(error) => return error,
	};
	let node = renamed_type_word_methods(node);
	let (node, likenesses) = match inherit(node) {
		Ok(inherited) => inherited,
		Err(error) => return error,
	};
	let traits = [shared_method_traits(&node), likenesses].concat();
	let shared: Vec<String> = traits.iter().filter_map(trait_operation).collect();
	let mut changing = Changing::default();
	let statics = static_members(&node);
	let setters = property_setters(&node);
	let node = lower_classes(node, &mut changing);
	let node = if setters.is_empty() { node } else { setter_calls(node, &setters) };
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

/// A method named like a type word (`double() := x*2`, P142: class methods are always allowed) or like a list
/// mutation (`pop() := items.pop()`) under the name `method·double`, so its calls never read as the conversion
/// `double(c)` or the list's own `xs.pop()`. A type word is renamed wherever it is called as a method; a list word only
/// on what holds an instance of a class defining it (a variable assigned one, `p:Stack`, self) and called bare in a
/// class body, so `s.items.pop()` and `xs.pop()` of lists stay list mutations
fn renamed_type_word_methods(node: Node) -> Node {
	let (mut names, mut classes) = (vec![], vec![]);
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		let clashing: Vec<String> = class_items(body).iter().filter_map(method_parts).map(|(name, _, _)| name)
			.filter(|name| crate::analyzer::type_word_kind(name).is_some() || is_library_method(name)).collect();
		if !clashing.is_empty() {
			classes.push(name.drop_meta().name());
		}
		names.extend(clashing);
	});
	if names.is_empty() {
		return node;
	}
	let mut instances: Vec<String> = instance_classes(&node, &classes).into_keys().collect();
	instances.extend([RECEIVER.to_string(), RECEIVER_ALIASES[0].to_string()]);
	with_method_names(node, &MethodNames { names, instances, classes }, false)
}

/// The variables known to hold an instance of one of `classes`, with its class: assigned a construction, annotated
/// `p:Point`, or iterating a list of constructions `for p in ps`
fn instance_classes(node: &Node, classes: &[String]) -> std::collections::HashMap<String, String> {
	let mut instances = std::collections::HashMap::new();
	node.visit(&mut |part| if let Node::Key(target, op, value) = part {
		// `p:Point = …`: the annotation says it
		if let (Node::Key(variable, Op::Colon, class), Op::Assign) = (target.drop_meta(), op) {
			if classes.contains(&class.drop_meta().name()) {
				instances.insert(variable.drop_meta().name(), class.drop_meta().name());
			}
		}
		let Node::Symbol(variable) = target.drop_meta() else { return };
		let class = match (op, value.drop_meta()) {
			(Op::Assign | Op::Define, _) => constructed_class(value),
			// `p:Point`, the annotation a symbol or a type
			(Op::Colon, Node::Symbol(_) | Node::Type { .. }) => Some(value.drop_meta().name()),
			_ => None,
		};
		if let Some(class) = class.filter(|class| classes.contains(class)) {
			instances.insert(variable.clone(), class);
		}
	});
	// `for p in ps {…}` over a list of constructions `ps = [Point(1, 2), …]`: p holds instances
	let list_class = |list: &Node| match list.drop_meta() {
		Node::List(items, Bracket::Square, _) if !items.is_empty() => {
			let first = constructed_class(&items[0])?;
			items.iter().all(|item| constructed_class(item).as_ref() == Some(&first)).then_some(first)
		}
		_ => None,
	};
	let mut instance_lists = std::collections::HashMap::new();
	node.visit(&mut |part| if let Node::Key(target, Op::Assign | Op::Define, value) = part {
		if let Some(class) = list_class(value).filter(|class| classes.contains(class)) {
			instance_lists.insert(target.drop_meta().name(), class);
		}
	});
	node.visit(&mut |part| if let Node::List(words, _, _) = part {
		if let [for_word, item, in_word, list, ..] = words.as_slice() {
			let class = instance_lists.get(&list.drop_meta().name()).cloned().or_else(|| list_class(list)).filter(|class| classes.contains(class));
			if let (true, Some(class)) = (for_word.drop_meta().name() == "for" && in_word.drop_meta().name() == "in", class) {
				instances.insert(item.drop_meta().name(), class);
			}
		}
	});
	instances
}

/// The class a construction `Point(1, 2)`, `Point{…}` builds (by its name, before the constructions are lowered)
fn constructed_class(value: &Node) -> Option<String> {
	match value.drop_meta() {
		Node::List(items, Bracket::Round, _) => items.first().map(|first| first.drop_meta().name()),
		Node::Key(class, Op::None | Op::Colon, _) => Some(class.drop_meta().name()),
		_ => None,
	}
}

/// `a + b` of an instance a whose class defines the operator's method (`plus`, Python's `__add__`, wiki/operator.md):
/// the method call `a.plus(b)`; an operation of such calls is an instance of the same class (`a + b + c`)
fn operator_calls(node: Node) -> Node {
	let mut methods: Vec<(String, Op, String)> = vec![];
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		for (method, _, _) in class_items(body).iter().filter_map(method_parts) {
			if let Some((op, names)) = OPERATOR_METHODS.iter().find(|(_, names)| names.contains(&method.as_str())) {
				if method != names[0] {
					crate::diagnostic::note_alias(&method, names[0]);
				}
				methods.push((name.drop_meta().name(), op.clone(), method));
			}
		}
	});
	if methods.is_empty() {
		return node;
	}
	let classes: Vec<String> = methods.iter().map(|(class, _, _)| class.clone()).collect();
	let instances = instance_classes(&node, &classes);
	with_operator_calls(node, &methods, &instances).0
}

/// The node with its operator calls, and the class of the instance it gives when it is one
fn with_operator_calls(node: Node, methods: &[(String, Op, String)], instances: &std::collections::HashMap<String, String>) -> (Node, Option<String>) {
	let recurse = |child: Node| with_operator_calls(child, methods, instances).0;
	match node {
		Node::Key(left, op, right) if OPERATOR_METHODS.iter().any(|(known, _)| *known == op) => {
			let (left, class) = with_operator_calls(*left, methods, instances);
			let right = recurse(*right);
			let class = class.or_else(|| match left.drop_meta() {
				Node::Symbol(variable) => instances.get(variable).cloned(),
				// `(a + b).x`: the parenthesized operation
				Node::List(items, Bracket::Round, _) if items.len() == 1 => with_operator_calls(items[0].clone(), methods, instances).1,
				other => constructed_class(other),
			});
			match class.as_ref().and_then(|class| methods.iter().find(|(owner, known, _)| owner == class && *known == op)) {
				Some((_, _, method)) => {
					let call = Node::List(vec![Node::Symbol(method.clone()), right], Bracket::Round, Separator::None);
					(Node::Key(Box::new(left), Op::Dot, Box::new(call)), class)
				}
				None => (Node::Key(Box::new(left), op, Box::new(right)), None),
			}
		}
		Node::List(items, Bracket::Round, separator) if items.len() == 1 => {
			let (item, class) = with_operator_calls(items[0].clone(), methods, instances);
			(Node::List(vec![item], Bracket::Round, separator), class)
		}
		other => (other.map_children(recurse), None),
	}
}

/// A name the library gives lists and texts too (`pop`, `sum`, `count`): a method of that name is one only on an
/// instance
fn is_library_method(name: &str) -> bool {
	crate::analyzer::is_list_mutating_method(name) || crate::library_words::is_library_word(name)
}

/// The clashing method names, and the variables holding instances of the classes defining them
struct MethodNames {
	names: Vec<String>,
	instances: Vec<String>,
	classes: Vec<String>,
}

fn method_name(name: &str) -> String {
	format!("{METHOD_PREFIX}{name}")
}

/// The clashing methods renamed where they are defined and called: `c.double`, `c.pop()`, and in a class body
/// `double()` and the definition `double() := …`
fn with_method_names(node: Node, methods: &MethodNames, in_class: bool) -> Node {
	let renamed = |name: &Node| match name.drop_meta() {
		Node::Symbol(word) if methods.names.contains(word) => Some(Node::Symbol(method_name(word))),
		_ => None,
	};
	let renamed_call = |call: &Node| match call.drop_meta() {
		Node::List(items, Bracket::Round, separator) if !items.is_empty() => {
			renamed(&items[0]).map(|name| Node::List([vec![name], items[1..].to_vec()].concat(), Bracket::Round, separator.clone()))
		}
		_ => None,
	};
	// a type word is a method on anything but a number (`3.double()` stays the conversion), a list word on an instance
	let calls_method = |receiver: &Node, member: &Node| {
		let name = match member.drop_meta() {
			Node::List(items, Bracket::Round, _) => items.first().map(|first| first.drop_meta().name()).unwrap_or_default(),
			other => other.name(),
		};
		match is_library_method(&name) {
			true => match receiver.drop_meta() {
				Node::Symbol(variable) => methods.instances.contains(variable),
				// `Stack([]).count()`, a construction
				Node::List(items, Bracket::Round, _) => items.first().is_some_and(|class| methods.classes.contains(&class.drop_meta().name())),
				Node::Key(class, Op::None | Op::Colon, _) => methods.classes.contains(&class.drop_meta().name()),
				_ => false,
			},
			false => !matches!(receiver.drop_meta(), Node::Number(_)),
		}
	};
	let recurse = |child: Node| with_method_names(child, methods, in_class);
	match node {
		Node::Type { name, body } => Node::Type { name, body: Box::new(with_method_names(*body, methods, true)) },
		Node::Key(receiver, Op::Dot, member) if calls_method(&receiver, &member) => {
			let member = renamed(&member).or_else(|| renamed_call(&member)).unwrap_or_else(|| recurse(*member));
			Node::Key(Box::new(recurse(*receiver)), Op::Dot, Box::new(member))
		}
		// `items.pop()` of a list: the member keeps its name, its arguments may call methods
		Node::Key(receiver, Op::Dot, member) => {
			let member = match *member {
				Node::List(items, Bracket::Round, separator) if !items.is_empty() => {
					Node::List(items[..1].iter().cloned().chain(items[1..].iter().cloned().map(recurse)).collect(), Bracket::Round, separator)
				}
				other => other,
			};
			Node::Key(Box::new(recurse(*receiver)), Op::Dot, Box::new(member))
		}
		Node::List(..) if in_class && renamed_call(&node).is_some() => {
			let Node::List(items, bracket, separator) = renamed_call(&node).expect("guarded") else { unreachable!("a call") };
			Node::List(items.into_iter().map(recurse).collect(), bracket, separator)
		}
		// `pop() := …`, or a getter `double := …`
		Node::Key(head, Op::Define, body) if in_class && (renamed(&head).is_some() || renamed_call(&head).is_some()) => {
			let head = renamed(&head).or_else(|| renamed_call(&head)).expect("guarded");
			Node::Key(Box::new(head), Op::Define, Box::new(recurse(*body)))
		}
		other => other.map_children(recurse),
	}
}

/// Rust's `impl Point { fn sum(&self) -> i32 {…} }` (and `impl Trait for Point {…}`): its functions are methods of
/// the class Point, the impl block itself goes
fn with_impls(node: Node) -> Node {
	let mut impls: Vec<(String, Vec<Node>)> = vec![];
	node.visit(&mut |part| if let Some((class, items)) = impl_block(part) {
		impls.push((class, items));
	});
	if impls.is_empty() {
		return node;
	}
	taking_in_impls(node, &impls)
}

fn taking_in_impls(node: Node, impls: &[(String, Vec<Node>)]) -> Node {
	match node {
		_ if impl_block(&node).is_some() => Node::Empty,
		Node::Type { name, body } => {
			let class = name.drop_meta().name();
			let added: Vec<Node> = impls.iter().filter(|(owner, _)| *owner == class).flat_map(|(_, items)| items.clone()).collect();
			match added.is_empty() {
				true => Node::Type { name, body },
				false => Node::Type { name, body: Box::new(Node::List([class_items(&body), added].concat(), Bracket::Curly, Separator::Semicolon)) },
			}
		}
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| taking_in_impls(item, impls)).filter(|item| !matches!(item, Node::Empty)).collect();
			Node::List(items, bracket, separator)
		}
		Node::Meta { node, data } => match taking_in_impls(*node, impls) {
			Node::Empty => Node::Empty,
			node => Node::Meta { node: Box::new(node), data },
		},
		other => other,
	}
}

/// `impl Point {…}`, `impl Display for Point {…}`: the class and the items of the block
fn impl_block(node: &Node) -> Option<(String, Vec<Node>)> {
	let Node::List(words, _, _) = node.drop_meta() else { return None };
	let (class, block) = match words.as_slice() {
		[word, class, block] if word.drop_meta().name() == IMPL_WORD => (class, block),
		// `impl Point {…}` of a declared Point reads like the spaced construction `Point {…}`
		[word, construction] if word.drop_meta().name() == IMPL_WORD => match construction.drop_meta() {
			Node::Key(class, Op::None | Op::Colon, block) => (class.as_ref(), block.as_ref()),
			_ => return None,
		},
		[word, _, for_word, class, block] if word.drop_meta().name() == IMPL_WORD && for_word.drop_meta().name() == "for" => (class, block),
		_ => return None,
	};
	let Node::Symbol(class) = class.drop_meta() else { return None };
	match block.drop_meta() {
		Node::List(_, Bracket::Curly, _) => Some((class.clone(), class_items(block))),
		_ => None,
	}
}

/// Kotlin's `p.copy(y = 5)`: a copy of p with those fields changed, `field_with(p, "y", 5)`
fn copies(node: Node) -> Node {
	let changed = |argument: &Node| match argument.drop_meta() {
		Node::Key(field, Op::Assign | Op::Colon, value) if matches!(field.drop_meta(), Node::Symbol(_)) => Some((field.drop_meta().name(), value.as_ref().clone())),
		_ => None,
	};
	match node {
		Node::Key(receiver, Op::Dot, member) => {
			let receiver = copies(*receiver);
			let member = copies(*member);
			let changes: Option<Vec<(String, Node)>> = match member.drop_meta() {
				Node::List(items, Bracket::Round, _) if items.len() > 1 && items[0].drop_meta().name() == COPY_WORD => items[1..].iter().map(changed).collect(),
				_ => None,
			};
			match changes {
				Some(changes) => changes.into_iter().fold(receiver, |object, (field, value)| {
					let call = vec![Node::Symbol(crate::library_words::FIELD_WITH.to_string()), object, Node::Text(field), value];
					Node::List(call, Bracket::Round, Separator::None)
				}),
				None => Node::Key(Box::new(receiver), Op::Dot, Box::new(member)),
			}
		}
		other => other.map_children(copies),
	}
}

/// Every class with the items of the mixins it takes in (`class Duck with Walker {…}`, `include Walker` in its body)
/// after its own, but those it defines itself; the mixin declarations themselves are no classes and go
fn with_mixins(node: Node) -> Result<Node, Node> {
	let mut mixins: Vec<(String, Vec<Node>)> = vec![];
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		if name.attribute(crate::wasp_parser::MIXIN_WORD).is_some() {
			mixins.push((name.drop_meta().name(), class_items(body)));
		}
	});
	if mixins.is_empty() {
		return Ok(node);
	}
	taking_in_mixins(node, &mixins)
}

fn taking_in_mixins(node: Node, mixins: &[(String, Vec<Node>)]) -> Result<Node, Node> {
	match node {
		Node::Type { name, .. } if name.attribute(crate::wasp_parser::MIXIN_WORD).is_some() => Ok(Node::Empty),
		Node::Type { name, body } => {
			let mut taken: Vec<String> = name.attribute(crate::wasp_parser::WITH_KEYWORD).map(|names| match names.drop_meta() {
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
			let name = match name.attribute(crate::wasp_parser::EXTENDS_KEYWORD) {
				Some(parent) => Node::Symbol(name.drop_meta().name()).with_attribute(crate::wasp_parser::EXTENDS_KEYWORD, parent.clone()),
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
fn included_mixin(item: &Node) -> Option<String> {
	let Node::List(words, _, _) = item.drop_meta() else { return None };
	let [word, mixin] = words.as_slice() else { return None };
	(word.drop_meta().name() == INCLUDE_WORD && matches!(mixin.drop_meta(), Node::Symbol(_))).then(|| mixin.drop_meta().name())
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
		// a block of one statement `{fn sum() …}` is that statement's words
		Node::List(words, Bracket::Curly, Separator::Space) if words.first().is_some_and(|first| matches!(first.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word))) || auto_property(words).is_some() || typed_field(words).is_some() => {
			vec![Node::List(words.clone(), Bracket::None, Separator::Space)]
		}
		Node::List(items, _, _) => items.clone(),
		Node::Empty => vec![],
		single => vec![single.clone()],
	};
	items.into_iter().map(without_modifiers).flat_map(|item| match item.drop_meta() {
		Node::List(words, _, _) if keyword_method(words).is_some() => keyword_method(words).into_iter().collect(),
		Node::List(words, _, _) if braced_method(words).is_some() => braced_method(words).into_iter().collect(),
		Node::List(words, _, _) if value_block(words).is_some() => value_block(words).into_iter().collect(),
		Node::List(words, _, _) if accessors(words).is_some() => accessors(words).unwrap_or_default(),
		Node::List(..) if included_mixin(&item).is_some() => vec![item],
		// Java's `int x`: the field x of type int; C#'s auto-property `int X { get; set; }` the field X
		Node::List(words, _, _) if typed_field(words).is_some() => typed_field(words).into_iter().collect(),
		Node::List(words, _, _) if auto_property(words).is_some() => auto_property(words).into_iter().collect(),
		// TypeScript's `twice(): number { … }`
		Node::Key(call, Op::Colon, typed_block) if typed_method(call, typed_block).is_some() => typed_method(call, typed_block).into_iter().collect(),
		Node::List(group, Bracket::None, _) => group.clone(),
		_ => vec![item],
	}).map(as_member).collect()
}

/// `def area() -> int {…}`, `fun area(): Int {…}`, `func area() {…}`: the method `area() := …` (with its result type)
fn keyword_method(words: &[Node]) -> Option<Node> {
	// Kotlin's expression body `fun sum() = x + y` defines as `:=` does; Java's `int sum() {…}` as C's
	match crate::declarations::keyword_definition(words).or_else(|| crate::declarations::c_function(words))? {
		Node::Key(head, Op::Assign, body) => Some(Node::Key(head, Op::Define, body)),
		definition => Some(definition),
	}
}

/// A property's getter and setter as methods: `get age() {…}` is the getter `age := …`, `set age(v) {…}` the setter
/// `age·set(v) := …`; the wasp form `age:{getter} set{setter}` has the new value as `it`
fn accessors(words: &[Node]) -> Option<Vec<Node>> {
	let [first, second] = words else { return None };
	let method = |name: &str, parameters: Vec<Node>, body: &Node| {
		let head = match parameters.is_empty() {
			true => Node::Symbol(name.to_string()),
			false => Node::List([vec![Node::Symbol(name.to_string())], parameters].concat(), Bracket::Round, Separator::None),
		};
		Node::Key(Box::new(head), Op::Define, Box::new(body.clone()))
	};
	let is_block = |node: &Node| matches!(node.drop_meta(), Node::List(_, Bracket::Curly, _));
	match (first.drop_meta(), second.drop_meta()) {
		// `get age() {…}`, `set age(v) {…}`
		(Node::Symbol(word), Node::List(parts, _, _)) if crate::wasp_parser::ACCESSOR_WORDS.contains(&word.as_str()) => {
			let [call, body] = parts.as_slice() else { return None };
			let Node::List(call, Bracket::Round, _) = call.drop_meta() else { return None };
			let name = call.first()?.drop_meta().name();
			let parameters: Vec<Node> = call[1..].iter().flat_map(|parameter| match parameter.drop_meta() {
				Node::List(group, Bracket::Round, _) => group.clone(),
				Node::Empty => vec![],
				_ => vec![parameter.clone()],
			}).collect();
			match word == "get" {
				true => Some(vec![method(&name, vec![], body)]),
				false => Some(vec![method(&format!("{name}{SETTER_SUFFIX}"), parameters, body)]),
			}
		}
		// `age:{2026 - birthday} set{birthday = 2026 - it}`
		(Node::Key(name, Op::Colon | Op::None, getter), Node::Key(set, Op::Colon | Op::None, setter)) if set.drop_meta().name() == "set" && is_block(getter) && is_block(setter) => {
			let name = name.drop_meta().name();
			// `it` is the new value; named apart, so no pass reads the block as a lambda of `it`
			let new_value = Node::Symbol(format!("{}{SETTER_SUFFIX}", crate::wasp_parser::IT_WORD));
			let setter = renamed_symbol(setter.as_ref().clone(), crate::wasp_parser::IT_WORD, &new_value);
			Some(vec![method(&name, vec![], getter), method(&format!("{name}{SETTER_SUFFIX}"), vec![new_value], &setter)])
		}
		_ => None,
	}
}

/// The properties the classes of the program give setters
fn property_setters(node: &Node) -> Vec<String> {
	let mut setters = vec![];
	node.visit(&mut |part| if let Node::Type { body, .. } = part {
		setters.extend(class_items(body).iter().filter_map(method_parts).filter_map(|(name, _, _)| name.strip_suffix(SETTER_SUFFIX).map(str::to_string)));
	});
	setters
}

fn renamed_symbol(node: Node, name: &str, replacement: &Node) -> Node {
	match node {
		Node::Symbol(symbol) if symbol == name => replacement.clone(),
		other => other.map_children(|child| renamed_symbol(child, name, replacement)),
	}
}

/// `p.age = v` of a property with a setter: `p = age·set(p, v)`
fn setter_calls(node: Node, setters: &[String]) -> Node {
	let recurse = |child: Node| setter_calls(child, setters);
	match node {
		Node::Key(target, Op::Assign, value) => match target.drop_meta() {
			Node::Key(receiver, Op::Dot, property) if matches!(receiver.drop_meta(), Node::Symbol(_)) && setters.contains(&property.drop_meta().name()) => {
				let setter = Node::Symbol(format!("{}{SETTER_SUFFIX}", property.drop_meta().name()));
				let call = Node::List(vec![setter, receiver.as_ref().clone(), recurse(*value)], Bracket::Round, Separator::None);
				Node::Key(receiver.clone(), Op::Assign, Box::new(call))
			}
			_ => Node::Key(Box::new(recurse(*target)), Op::Assign, Box::new(recurse(*value))),
		},
		other => other.map_children(recurse),
	}
}

/// A member without the words that change nothing in wasp: `mutating func f() {…}` is `func f() {…}`, Swift's
/// `var count = 0` the field `count = 0`
fn without_modifiers(item: Node) -> Node {
	let Node::List(words, bracket, separator) = item.drop_meta().clone() else { return item };
	let is_modifier = |word: &Node| matches!(word.drop_meta(), Node::Symbol(word) if crate::wasp_parser::MEMBER_MODIFIERS.contains(&word.as_str()) || FIELD_KEYWORDS.contains(&word.as_str()));
	let kept: Vec<Node> = words.iter().skip_while(|word| is_modifier(word)).cloned().collect();
	if let Some(kept_word) = kept.first().map(leading_name).filter(|_| kept.len() < words.len()) {
		let modifiers: Vec<String> = words[..words.len() - kept.len()].iter().map(|word| word.drop_meta().name()).collect();
		if !modifiers.iter().any(|word| SILENT_MODIFIERS.contains(&word.as_str())) {
			crate::diagnostic::note_alias(&format!("{} {kept_word}", modifiers.join(" ")), &kept_word);
		}
	}
	match kept.len() {
		length if length == words.len() => item,
		1 => kept.into_iter().next().expect("one"),
		_ => Node::List(kept, bracket, separator),
	}
}

/// The name a member starts with: `count` of `count = 0`, `up` of `up() {…}`
pub(crate) fn leading_name(member: &Node) -> String {
	match member.drop_meta() {
		Node::Key(left, _, _) => leading_name(left),
		Node::List(items, _, _) => items.first().map(leading_name).unwrap_or_default(),
		other => other.name(),
	}
}

/// Java's and C#'s field `int x`: the field `x:int`
fn typed_field(words: &[Node]) -> Option<Node> {
	let [field_type, name] = words else { return None };
	let is_type = crate::analyzer::type_word_kind(&field_type.drop_meta().name()).is_some();
	(is_type && matches!(name.drop_meta(), Node::Symbol(_))).then(|| Node::Key(Box::new(name.clone()), Op::Colon, Box::new(field_type.clone())))
}

/// C#'s auto-property `int X { get; set; }` (or `{ get; init; }`): the field `X:int`
fn auto_property(words: &[Node]) -> Option<Node> {
	let [field_type, name, accessors] = words else { return None };
	let Node::List(accessors, Bracket::Curly, _) = accessors.drop_meta() else { return None };
	let only_accessors = !accessors.is_empty() && accessors.iter().all(|accessor| matches!(accessor.drop_meta().name().as_str(), "get" | "set" | "init"));
	only_accessors.then(|| typed_field(&[field_type.clone(), name.clone()])).flatten()
}

/// TypeScript's method with its result type, `twice(): number { … }`: `twice():number := {…}`
fn typed_method(call: &Node, typed_block: &Node) -> Option<Node> {
	let is_call = matches!(call.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))));
	let Node::List(parts, _, _) = typed_block.drop_meta() else { return None };
	let [result_type, block] = parts.as_slice() else { return None };
	let is_block = matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _));
	(is_call && is_block).then(|| Node::Key(Box::new(Node::Key(Box::new(call.clone()), Op::Colon, Box::new(result_type.clone()))), Op::Define, Box::new(block.clone())))
}

/// Python's class attribute: a field `count = 0` read as `Counter.count` is the class's own, shared by its instances
/// (the static `static count = 0`)
fn with_class_attributes(node: Node) -> Node {
	let mut qualified: Vec<(String, String)> = vec![];
	node.visit(&mut |part| if let Node::Key(class, Op::Dot, member) = part {
		if let (Node::Symbol(class), Node::Symbol(member)) = (class.drop_meta(), member.drop_meta()) {
			qualified.push((class.clone(), member.clone()));
		}
	});
	if qualified.is_empty() {
		return node;
	}
	class_attributes(node, &qualified)
}

fn class_attributes(node: Node, qualified: &[(String, String)]) -> Node {
	match node {
		Node::Type { name, body } => {
			let class = name.drop_meta().name();
			// a field with its value `count = 0` (a parameter named like the class reads `photo.width` of an instance)
			let has_value = |item: &Node| matches!(item.drop_meta(), Node::Key(_, Op::Assign, _));
			let is_attribute = |item: &Node| method_parts(item).is_none() && has_value(item) && field_name(item).is_some_and(|field| qualified.contains(&(class.clone(), field)));
			let items = class_items(&body);
			if !items.iter().any(|item| is_attribute(item) && !is_static(item)) {
				return Node::Type { name, body };
			}
			let items = items.into_iter().map(|item| match is_attribute(&item) {
				true => item.with_attribute(crate::wasp_parser::STATIC_KEYWORD, Node::True),
				false => item,
			}).collect();
			Node::Type { name, body: Box::new(Node::List(items, Bracket::Curly, Separator::Semicolon)) }
		}
		other => other.map_children(|child| class_attributes(child, qualified)),
	}
}

/// Every class's constructor as `init(…){…}` (P162): one named by another language (`constructor(x)`, `__init__`,
/// CONSTRUCTOR_ALIASES) or like its class (Java's `Point(int x, int y) {…}`), with a got-it note; an alias the program
/// also calls as a method (`p.new(2)`) stays a method
fn with_init_constructors(node: Node, called: &std::collections::HashSet<String>) -> Node {
	match node {
		Node::Type { name, body } => {
			let class = name.drop_meta().name();
			let is_alias = |item: &Node| constructor_alias(item).filter(|alias| *alias == class || (CONSTRUCTOR_ALIASES.contains(&alias.as_str()) && !called.contains(alias)));
			let items: Vec<Node> = class_items(&body);
			if !items.iter().any(|item| is_alias(item).is_some()) {
				return Node::Type { name, body };
			}
			let items = items.into_iter().map(|item| match is_alias(&item) {
				Some(alias) => {
					crate::diagnostic::note_alias(&alias, CONSTRUCTOR_WORD);
					as_init(item)
				}
				None => item,
			}).collect();
			Node::Type { name, body: Box::new(Node::List(items, Bracket::Curly, Separator::Semicolon)) }
		}
		other => other.map_children(|child| with_init_constructors(child, called)),
	}
}

/// The name a constructor-like member is written with: `constructor` of `constructor(x){…}`, `Point` of `Point(x) := …`
fn constructor_alias(item: &Node) -> Option<String> {
	match item.drop_meta() {
		Node::Key(word, Op::None | Op::Colon, block) if matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _)) && constructor_parameters(word).is_some() => Some(leading_name(word)),
		_ => method_parts(item).map(|(method, _, _)| method),
	}
}

/// A constructor-like member as the constructor `init(…){…}`
fn as_init(item: Node) -> Node {
	let (parameters, body) = match (constructor_parts(&item), method_parts(&item)) {
		(Some((parameters, body)), _) => (parameters, body.clone()),
		(None, Some((_, parameters, body))) => (parameters, curly(body)),
		(None, None) => return item,
	};
	let word = Node::List([vec![Node::Symbol(CONSTRUCTOR_WORD.to_string())], parameters].concat(), Bracket::Round, Separator::None);
	Node::Key(Box::new(word), Op::None, Box::new(body))
}

/// The names the program calls as methods, `new` of `p.new(2)` and of Rust's `Point::new(1, 2)`
fn method_calls_named(node: &Node) -> std::collections::HashSet<String> {
	let mut names = std::collections::HashSet::new();
	node.visit(&mut |part| if let Node::Key(_, Op::Dot | Op::SafeDot | Op::Scope, member) = part {
		names.insert(leading_name(member));
	});
	names
}

/// Every class body as its members (class_items): Java's `int x` fields and C#'s auto-properties are fields also in a
/// class without methods, which no later pass splits
fn with_members(node: Node) -> Node {
	match node {
		Node::Type { name, body } if name.attribute(crate::wasp_parser::MIXIN_WORD).is_none() => {
			let members = Node::List(class_items(&body), Bracket::Curly, Separator::Semicolon);
			let changed = members.serialize() != Node::List(class_items_as_written(&body), Bracket::Curly, Separator::Semicolon).serialize();
			Node::Type { name, body: Box::new(if changed { members } else { *body }) }
		}
		other => other.map_children(with_members),
	}
}

/// The items of a class body as written, its `;` groups flattened, nothing read into them
fn class_items_as_written(body: &Node) -> Vec<Node> {
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

/// Java's and C#'s `Point p = new Point(3, 4);`, a variable declared with a class as its type: `p:Point = Point(3, 4)`
/// (the type stays: a call may pick its overload by it, D10)
fn class_typed_declarations(node: Node) -> Node {
	let mut classes = vec![];
	node.visit(&mut |part| if let Node::Type { name, .. } = part {
		classes.push(name.drop_meta().name());
	});
	if classes.is_empty() {
		return node;
	}
	declared_with_classes(node, &classes)
}

fn declared_with_classes(node: Node, classes: &[String]) -> Node {
	match node {
		Node::List(words, _, _) if words.len() == 2 && classes.contains(&words[0].drop_meta().name()) && matches!(words[1].drop_meta(), Node::Key(variable, Op::Assign, _) if matches!(variable.drop_meta(), Node::Symbol(_))) => {
			let Node::Key(variable, _, value) = words[1].drop_meta().clone() else { unreachable!("guarded") };
			let typed = Node::Key(variable, Op::Colon, Box::new(words[0].clone()));
			Node::Key(Box::new(typed), Op::Assign, Box::new(declared_with_classes(*value, classes)))
		}
		other => other.map_children(|child| declared_with_classes(child, classes)),
	}
}

/// JavaScript's `sum() { return … }`: the method `sum() := {…}`
fn braced_method(words: &[Node]) -> Option<Node> {
	let [call, block] = words else { return None };
	let is_call = matches!(call.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if !is_constructor_word(name)));
	let is_block = matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _));
	(is_call && is_block).then(|| Node::Key(Box::new(call.clone()), Op::Define, Box::new(block.clone())))
}

/// A method as other languages write it, as a class member: Python's explicit `self` first parameter dropped (the
/// method gets its receiver anyway), a method named by an operator's glyph under the operator's english name
fn as_member(definition: Node) -> Node {
	let Node::Key(head, Op::Define, body) = definition.drop_meta().clone() else { return definition };
	// `sum(self):i32 := …`: the head under its result type
	if let Node::Key(call, Op::Colon, result_type) = head.drop_meta().clone() {
		if matches!(call.drop_meta(), Node::List(_, Bracket::Round, _)) {
			return match as_member(Node::Key(call, Op::Define, body)) {
				Node::Key(call, Op::Define, body) => Node::Key(Box::new(Node::Key(call, Op::Colon, result_type)), Op::Define, body),
				constructor => constructor,
			};
		}
	}
	let Node::List(call, Bracket::Round, separator) = head.drop_meta().clone() else { return definition };
	let Some((name, parameters)) = call.split_first() else { return definition };
	let parameters: Vec<Node> = parameters.iter().flat_map(|parameter| match parameter.drop_meta() {
		Node::List(group, Bracket::Round, _) => group.clone(),
		Node::Empty => vec![],
		_ => vec![parameter.clone()],
	}).collect();
	// `self`, `self: Self`, Rust's `&self`
	let is_self = |parameter: &Node| match parameter.drop_meta() {
		Node::Symbol(word) => word == RECEIVER,
		Node::Key(word, Op::Colon, _) => word.drop_meta().name() == RECEIVER,
		Node::Key(empty, _, word) => matches!(empty.drop_meta(), Node::Empty) && word.drop_meta().name() == RECEIVER,
		_ => false,
	};
	let parameters = match parameters.first() {
		Some(first) if is_self(first) => parameters[1..].to_vec(),
		_ => parameters,
	};
	let name = glyph_method(&name.drop_meta().name()).map_or(name.clone(), |method| Node::Symbol(method.to_string()));
	Node::Key(Box::new(Node::List([vec![name], parameters].concat(), Bracket::Round, separator)), Op::Define, body)
}

/// A body as the block `{…}` a constructor has
fn curly(body: Node) -> Node {
	match body {
		Node::List(items, Bracket::Curly, separator) => Node::List(items, Bracket::Curly, separator),
		Node::List(items, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => Node::List(items, Bracket::Curly, separator),
		other => Node::List(vec![other], Bracket::Curly, Separator::Semicolon),
	}
}

/// `init {…}` (spaced) and `init(name) {…}`: the constructor `init{…}`, `init(name):{…}`
fn value_block(words: &[Node]) -> Option<Node> {
	let [word, block] = words else { return None };
	let is_block = matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _));
	(is_block && constructor_parameters(word).is_some()).then(|| Node::Key(Box::new(word.clone()), Op::None, Box::new(block.clone())))
}

fn is_constructor_word(name: &str) -> bool {
	name == CONSTRUCTOR_WORD || CONSTRUCTOR_ALIASES.contains(&name)
}

/// The parameters `init` or `init(name)` declares (or an alias's, `value(name)`)
fn constructor_parameters(word: &Node) -> Option<Vec<Node>> {
	match word.drop_meta() {
		Node::Symbol(name) if is_constructor_word(name) => Some(vec![]),
		// `init(a, b)`, `init (n)`: the parameters may come as one group
		Node::List(call, Bracket::Round, _) if matches!(call.first().map(Node::drop_meta), Some(Node::Symbol(name)) if is_constructor_word(name)) => {
			Some(call[1..].iter().flat_map(|parameter| match parameter.drop_meta() {
				Node::List(group, Bracket::Round, _) => group.clone(),
				_ => vec![parameter.clone()],
			}).collect())
		}
		_ => None,
	}
}

/// The parameters and statements of the constructor `init{…}`, `init(name){…}`
fn constructor_parts(item: &Node) -> Option<(Vec<Node>, &Node)> {
	match item.drop_meta() {
		// glued `init{…}` arrives as the key `init:{…}`
		Node::Key(word, Op::None | Op::Colon, block) if matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _)) => Some((constructor_parameters(word)?, block)),
		_ => None,
	}
}

fn constructor_body(item: &Node) -> Option<&Node> {
	constructor_parts(item).map(|(_, body)| body)
}

/// The fields a constructor body sets, `id = …` or `this.id = …`
/// (`this.x = x` sets the field x also when a parameter is called x; a bare `n = …` of a parameter n sets the parameter)
fn fields_set(body: &Node, parameters: &[String]) -> Vec<String> {
	let mut names = vec![];
	body.visit(&mut |part| if let Node::Key(target, Op::Assign, _) = part {
		let name = match target.drop_meta() {
			Node::Symbol(name) if !parameters.contains(name) => Some(name.clone()),
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
fn method_calls(node: Node, called: &[String], changing: &Changing) -> Node {
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
	let item = |pair: Node, position: i64| Node::Key(Box::new(pair), Op::Hash, Box::new(Node::int(position)));
	match receiver.drop_meta() {
		// `s.pop()`: the pair (value, changed object) of the call, the object stored back, the value given
		Node::Symbol(_) if changing.giving_value.contains(&name) => {
			let pair = Node::Symbol(format!("{name}{RESULT_SUFFIX}"));
			Node::List(vec![
				Node::Key(Box::new(pair.clone()), Op::Assign, Box::new(call)),
				Node::Key(Box::new(receiver), Op::Assign, Box::new(item(pair.clone(), 2))),
				item(pair, 1),
			], Bracket::Round, Separator::Semicolon)
		}
		_ if changing.giving_value.contains(&name) => item(call, 1),
		Node::Symbol(_) if changing.itself.contains(&name) => Node::Key(Box::new(receiver), Op::Assign, Box::new(call)),
		_ => call,
	}
}

/// The methods changing their object: those giving it back (`inc() := n += 1`) and those giving a value besides
/// (`pop() := items.pop()`), as the pair [value, object]
#[derive(Default)]
struct Changing {
	itself: Vec<String>,
	giving_value: Vec<String>,
}

impl Changing {
	fn is_empty(&self) -> bool {
		self.itself.is_empty() && self.giving_value.is_empty()
	}

	fn contains(&self, name: &String) -> bool {
		self.itself.contains(name) || self.giving_value.contains(name)
	}
}

fn lower_classes(node: Node, changing: &mut Changing) -> Node {
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
fn split_class(node: &Node, changing: &mut Changing) -> Option<(Node, Vec<Node>)> {
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
		for added in fields_set(body, &parameter_names) {
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
		match changes {
			Change::Itself => changing.itself.push(method_name),
			Change::GivingValue => changing.giving_value.push(method_name),
			Change::None => return with_result_type(function, result_type(method)),
		}
		function // it gives its changed object
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
fn function(members: &Members, method: &str, parameters: Vec<Node>, body: Node) -> (Node, Change) {
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
	let receiver = Node::Symbol(RECEIVER.to_string());
	let changes = match changes_receiver(&body) {
		false => Change::None,
		true if gives_value(&body) => Change::GivingValue,
		true => Change::Itself,
	};
	let body = match changes {
		Change::None => body,
		Change::Itself => Node::List(vec![body, receiver], Bracket::None, Separator::Semicolon),
		Change::GivingValue => {
			let result = Node::Symbol(format!("{method}{VALUE_SUFFIX}"));
			let pair = Node::List(vec![result.clone(), receiver], Bracket::Square, Separator::Space);
			Node::List(vec![Node::Key(Box::new(result), Op::Assign, Box::new(body)), pair], Bracket::None, Separator::Semicolon)
		}
	};
	let receiver = Node::Key(Box::new(Node::Symbol(RECEIVER.to_string())), Op::Colon, Box::new(Node::Symbol(class.to_string())));
	let head = Node::List([vec![Node::Symbol(method.to_string()), receiver], parameters].concat(), Bracket::Round, Separator::None);
	(Node::Key(Box::new(head), Op::Define, Box::new(body)), changes)
}

/// What a method does to its object: nothing, change it (giving it back), or change it and give a value
#[derive(PartialEq)]
enum Change {
	None,
	Itself,
	GivingValue,
}

/// Does a method body that changes its object end in a value of its own: `items.pop()`, not `n += 1` or `items.add(x)`
fn gives_value(body: &Node) -> bool {
	let last = match body.drop_meta() {
		Node::List(statements, Bracket::Curly, _) | Node::List(statements, Bracket::None, Separator::Semicolon | Separator::Newline) => statements.last(),
		_ => Some(body),
	};
	let Some(last) = last else { return false };
	match last.drop_meta() {
		Node::Key(_, op, _) if matches!(op, Op::Assign | Op::Define | Op::Inc | Op::Dec) || op.is_compound_assign() => false,
		// a body giving its object back already (a constructor)
		Node::Symbol(name) if name == RECEIVER => false,
		Node::Key(_, Op::Dot, call) => !mutating_call(call) || matches!(call.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|word| GIVING_MUTATIONS.contains(&word.drop_meta().name().as_str()))),
		_ => true,
	}
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
