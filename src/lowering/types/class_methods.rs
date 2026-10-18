//! Methods in a class body (issue #14, notes/classes.md): `class person{name; greet() := "hi " + name}` is the class
//! `person{name}` and the function `greet(self:person) := "hi " + self.name`. In a method body a bare field name and
//! `self`/`this` read the receiver; `p.greet()`, `greet(p)` and, for a method without parameters, `p.area` call it.
//! A method name that several classes define is declared as an implicit trait, so a call picks the class's own method
//! by the static type of its receiver (traits.rs witnesses).
//! `class dog extends animal {…}` (P117) gives dog the fields and methods of animal, its own ones override them, and
//! declares `dog like animal`, so a dog is accepted where an animal is wanted.

use super::nodes::{any_part, arguments_rewritten, assign, call, grouped_parameters, if_then, is_place, key, Counter};
use crate::words::COPY_WORD;
use crate::words::SELF_WORD;
use crate::words::{GLOBAL_WORD, MAP_WORD, RETURN_WORD, SUM_WORD};
mod components;
use components::*;
mod destructuring;
pub(crate) use destructuring::*;
mod instances;
pub(crate) use instances::*;
mod members;
use members::*;

mod inheritance;
mod json;
mod operators;
mod other_languages;
mod outside_methods;
pub use json::*;
use outside_methods::*;
mod method_aliases;
use method_aliases::*;
mod statics;
use crate::node::{leading_name, parens, symbol, text, Bracket, Node, Separator};
use crate::operators::{operator_method, Op, OPERATOR_METHODS};
use inheritance::*;
use operators::*;
pub(crate) use operators::{is_library_method, ClassFields};
use other_languages::*;
use statics::*;

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
use crate::warp_parser::CONSTRUCTOR_WORD;
/// A property's setter `set age(v) {…}` is the method `age·set(self, v)`, run by `p.age = v`
const SETTER_SUFFIX: &str = "·set";
/// The value a method changing its object gives besides it, `pop·value` in the method and `pop·result` of a call, and
/// the list mutations that give one
const VALUE_SUFFIX: &str = "·value";
const RESULT_SUFFIX: &str = "·result";
/// The value a class cast `s as Circle` checks, and its stand-in in the check's source text
const CAST_VALUE: &str = "cast·value";
const CAST_PLACEHOLDER: &str = "cast_placeholder";
const GIVING_MUTATIONS: [&str; 2] = ["pop", "remove"];
/// Member modifiers that may mean something in warp, so they get no note that warp needs them not
const SILENT_MODIFIERS: [&str; 1] = ["async"];
/// The constructor names of other languages, aliases of `init` (P162): warp's old `value`, JavaScript, Python, Ruby,
/// PHP, VB.NET, Delphi, Rust; besides a method named like its class (C++, Java, C#)
const CONSTRUCTOR_ALIASES: [&str; 8] =
	["value", "constructor", "__init__", "initialize", "__construct", "New", "Create", "new"];
/// The methods that give a class its text, equality and order (the witnesses of Printable, Equatable, Comparable),
/// and how other languages name them
const WITNESS_METHODS: [(&str, &[&str]); 3] = [
	("text", &["toString", "to_s", "__str__", "ToString", "String"]),
	(crate::traits::EQUALS, &["Equals", "__eq__", "equal"]),
	(crate::traits::COMPARE, &["compareTo", "CompareTo", "cmp"]),
];
/// Ruby's construction `Point.new(1, 2)`
const RUBY_NEW_WORD: &str = "new";
/// Go's function keyword, also of a method `func (p Point) Sum() int {…}`
const GO_FUNCTION_WORD: &str = "func";
/// Rust's block of methods of a type, `impl Point {…}`
const IMPL_WORD: &str = "impl";
/// lib/units.warp's `q.to("km/h")`: a run-time quantity in other units
const CONVERSION_METHOD: &str = "to";
/// lib/units.warp's check that two quantities measure the same, and a quantity's amount in base units
const SAME_QUANTITY: &str = "same_quantity";
const QUANTITY_AMOUNT: &str = "amount";
/// The sum of a list of run-time quantities folds their `plus` (quantities_reduced)
const REDUCE_WORD: &str = "reduce";
const REDUCED_NAMES: [&str; 2] = ["quantity·sum", "quantity·item"];
/// P212: a class defining the method of only one of these operators has it serve the other too
const INTERCHANGEABLE_OPERATORS: [(Op, Op); 2] = [(Op::Similar, Op::Rough), (Op::Rough, Op::Similar)];
/// The run-time choice of an interval-field method by the receiver's class (dispatched_by_class)
const DISPATCH_TEMPLATE: &str = "if SELF_WORD is CLASS then METHOD else OTHERWISE";
/// Ruby's `include Walker` in a class body takes in a mixin
const INCLUDE_WORD: &str = "include";
/// The prefix of a method named like an interval's field: `method·low` (a prefix: a name `low·T` reads as a witness)
const METHOD_PREFIX: &str = "method·";

/// A method named by an operator's glyph, `+(o) := …`: the operator's method `plus`
fn glyph_method(name: &str) -> Option<&'static str> {
	OPERATOR_METHODS.iter().find(|(op, _)| op.as_str() == name).map(|(_, names)| names[0])
}

/// The function a class's `init{…}` block becomes, `person·init(self:person)`: every construction is passed through it
pub fn constructor_name(class: &str) -> String {
	format!("{class}·{CONSTRUCTOR_WORD}")
}

pub fn lower(node: Node) -> Node {
	let node = positional_braces(ruby_constructions(copies(with_impls(node))));
	let node = with_init_constructors(node.clone(), &method_calls_named(&node));
	let node = with_witness_methods(node);
	let node = with_members(node);
	let node = with_class_attributes(class_typed_declarations(node));
	let node = rendered_components(positional_fields(destructurings(from_objects(node))));
	let node = operator_calls(node);
	let node = match with_mixins(node) {
		Ok(node) => node,
		Err(error) => return error,
	};
	let node = method_aliases(node);
	let node = renamed_interval_field_methods(node);
	let (node, likenesses) = match inherit(node) {
		Ok(inherited) => inherited,
		Err(error) => return error,
	};
	let node = outside_receiver_reads(node);
	let traits = [shared_method_traits(&node), likenesses].concat();
	let shared: Vec<String> = traits.iter().filter_map(trait_operation).collect();
	let mut changing = Changing::default();
	let statics = static_members(&node);
	let setters = property_setters(&node);
	let instance_fields = class_typed_fields(&node);
	let node = lower_classes(node, &mut changing);
	let node = if setters.is_empty() { node } else { setter_calls(node, &setters) };
	let node = if statics.is_empty() { node } else { static_reads(node, &statics) };
	// `s.area`, `s.scaled(3)` of a shared method: the call `area(s)` before traits rename each class's method;
	// `c.inc()` of a method changing its object: `c = inc(c)` (P116)
	let node = if shared.is_empty() && changing.is_empty() {
		node
	} else {
		method_calls(
			node,
			&MethodCalls { called: &shared, changing: &changing, instance_fields: &instance_fields },
			&Counter::starting_at(1),
		)
	};
	match (traits.is_empty(), node) {
		(true, node) => node,
		(false, Node::List(items, bracket, separator)) if separator != Separator::Space => {
			Node::List([traits, items].concat(), bracket, separator)
		}
		(false, node) => Node::List([traits, vec![node]].concat(), Bracket::None, Separator::Semicolon),
	}
}

/// A class component's method giving its markup (Lit, React classes); other frameworks' names for it, with a note:
/// Backbone's and Mithril's `view`, Angular's `template`, Flutter's `build`
const RENDER_WORD: &str = "render";
const RENDER_ALIASES: [&str; 3] = ["view", "template", "build"];

/// `object as Point`, `parse_json(t) as Point`, `Point.from_json(t)`: the instance of the object's fields,
/// `(Point·from = object; Point(Point·from.x, Point·from.y))`; a field of a class type is built from its path
/// (`Line(Point(Line·from.a.x, …), …)`: a construction's arguments are its fields' data, no statements)
fn from_objects(node: Node) -> Node {
	let fields = class_fields(&node);
	if fields.is_empty() {
		return node;
	}
	with_objects_as_instances(node, &fields)
}

/// Each class's fields in order, with their type names
pub(crate) fn class_fields(node: &Node) -> ClassFields {
	let mut fields = std::collections::HashMap::new();
	node.visit(&mut |part| {
		if let Node::Type { name, body } = part {
			let typed = field_items(body).into_iter().map(|(_, name, type_name)| (name, type_name)).collect();
			fields.insert(name.drop_meta().name(), typed);
		}
	});
	fields
}

/// Each field of a class body: its name, type name (empty when untyped) and default value (`legs: int = 4`)
pub(crate) fn field_declarations(body: &Node) -> Vec<(String, String, Option<Node>)> {
	field_items(body)
		.into_iter()
		.map(|(item, name, type_name)| {
			let default = match item.drop_meta() {
				Node::Key(_, Op::Assign, value) => Some(value.as_ref().clone()),
				_ => None,
			};
			(name, type_name, default)
		})
		.collect()
}

/// Each field of a class body as written, with its name and type name (empty when untyped)
pub(crate) fn field_items(body: &Node) -> Vec<(Node, String, String)> {
	class_items(body)
		.into_iter()
		.filter(|item| method_parts(item).is_none())
		.filter_map(|item| {
			let name = declared_field_name(&item)?;
			let type_name = field_type_name(&item);
			Some((item, name, type_name))
		})
		.collect()
}

/// The fields annotated `@attribute(value)` (`@was(nick) name: text`): each name and its value
pub(crate) fn fields_marked(body: &Node, attribute: &str) -> Vec<(String, Node)> {
	class_items(body)
		.iter()
		.filter_map(|item| Some((declared_field_name(item)?, field_attribute(item, attribute)?.clone())))
		.collect()
}

/// The annotation on a field or on its name (`@was(nick) name: text` annotates the word name)
fn field_attribute<'a>(item: &'a Node, attribute: &str) -> Option<&'a Node> {
	item.attribute(attribute).or_else(|| match item.drop_meta() {
		Node::Key(field, Op::Colon | Op::Assign, _) => field_attribute(field, attribute),
		_ => None,
	})
}

/// Each class's parent and the names of its own fields and methods (`class Circle extends Shape { r: int }`)
pub(crate) fn class_members(node: &Node) -> std::collections::HashMap<String, (Option<String>, Vec<String>)> {
	let mut classes = std::collections::HashMap::new();
	node.visit(&mut |part| {
		if let Node::Type { name, body } = part {
			let parent = name.attribute(crate::warp_parser::EXTENDS_KEYWORD).map(|parent| parent.drop_meta().name());
			classes.insert(name.drop_meta().name(), (parent, class_items(body).iter().filter_map(item_name).collect()));
		}
	});
	classes
}

/// A class's parent and its own field and method names in declared order (reflection.rs), without constructors and
/// the methods the lowering derives (`age·set`)
pub(crate) struct ClassLayout {
	pub parent: Option<String>,
	pub fields: Vec<String>,
	pub methods: Vec<String>,
}

pub(crate) fn class_layouts(node: &Node) -> std::collections::HashMap<String, ClassLayout> {
	let mut layouts = declared_layouts(node);
	let outside = crate::declarations::extension_methods(node).into_iter().chain(first_parameter_methods(node));
	for (class, method) in outside.collect::<Vec<_>>() {
		if let Some(layout) = layouts.get_mut(&class).filter(|layout| !layout.methods.contains(&method)) {
			layout.methods.push(method);
		}
	}
	layouts
}

/// The layouts of the classes as declared, without the methods defined outside their bodies
pub(crate) fn declared_layouts(node: &Node) -> std::collections::HashMap<String, ClassLayout> {
	let mut layouts = std::collections::HashMap::new();
	node.visit(&mut |part| {
		if let Node::Type { name, body } = part {
			let parent = name.attribute(crate::warp_parser::EXTENDS_KEYWORD).map(|parent| parent.drop_meta().name());
			let class = name.drop_meta().name();
			let (mut fields, mut methods) = (vec![], vec![]);
			for item in class_items(body) {
				match method_parts(&item) {
					Some((method, _, _)) if method != CONSTRUCTOR_WORD && method != class && !method.contains('·') => {
						methods.push(method)
					}
					Some(_) => {}
					None => fields.extend(declared_field_name(&item)),
				}
			}
			layouts.insert(class, ClassLayout { parent, fields, methods });
		}
	});
	layouts
}

/// The (class, function) pairs of the definitions outside class bodies whose first parameter is declared a class:
/// `def x(p:P)` is P's method as much as a class-body `def x()` (card one-method-form)
fn first_parameter_methods(node: &Node) -> Vec<(String, String)> {
	fn collect(node: &Node, methods: &mut Vec<(String, String)>) {
		match node.drop_meta() {
			Node::Type { .. } => {}
			Node::Key(head, Op::Assign | Op::Define, body) => {
				methods.extend(first_parameter_class(head));
				collect(body, methods);
			}
			Node::Key(left, _, right) => {
				collect(left, methods);
				collect(right, methods);
			}
			Node::List(items, _, _) => items.iter().for_each(|item| collect(item, methods)),
			_ => {}
		}
	}
	let mut methods = vec![];
	collect(&crate::declarations::lower_c_functions(node.clone()), &mut methods);
	methods
}

/// `(P, x)` of the head `x(p:P)`
fn first_parameter_class(head: &Node) -> Option<(String, String)> {
	let (_, class) = first_parameter(head)?;
	Some((class, leading_name(head)))
}

/// The variable holding what is taken apart: `{x, y} = p` is `(parts·from = p; x = parts·from.x; y = parts·from.y)`
const PARTS_WORD: &str = "parts·from";

/// A method named like an interval's field (`low() := …`) under the name `method·low`, so its calls never read as the
/// field of an uncertain number: renamed on what holds an instance of a class defining it (a variable assigned one,
/// `p:Stack`, self) and called bare in a class body. Methods named like type or library words (`double()`, `pop()`)
/// keep their names and join the generic function, whose fallback is the library word (generics.rs)
fn renamed_interval_field_methods(node: Node) -> Node {
	let clashes =
		|name: &String| !is_witness_method(name) && crate::numbers::uncertain::INTERVAL_FIELDS.contains(&name.as_str());
	let mut defined_by: Vec<(String, String)> = vec![];
	node.visit(&mut |part| {
		if let Node::Type { name, body } = part {
			let class = name.drop_meta().name();
			let methods: Vec<String> =
				class_items(body).iter().filter_map(method_parts).map(|(name, _, _)| name).collect();
			defined_by.extend(methods.into_iter().filter(clashes).map(|method| (method, class.clone())));
		}
	});
	if defined_by.is_empty() {
		return node;
	}
	let names: Vec<String> = defined_by.iter().map(|(method, _)| method.clone()).collect();
	let mut classes: Vec<String> = defined_by.iter().map(|(_, class)| class.clone()).collect();
	classes.dedup();
	let mut instances: Vec<String> = instance_classes(&node, &classes).into_keys().collect();
	instances.extend([SELF_WORD.to_string(), RECEIVER_ALIASES[0].to_string()]);
	with_method_names(node, &MethodNames { names, instances, classes, defined_by }, false)
}

/// The clashing method names, and the variables holding instances of the classes defining them
struct MethodNames {
	names: Vec<String>,
	instances: Vec<String>,
	classes: Vec<String>,
	/// (method, class) of every renamed method
	defined_by: Vec<(String, String)>,
}

/// The receiver of no known class calls an interval-field method (`x.low`) that classes define: the class's method
/// when x is one of them at run time, else the field (card class-method)
fn dispatched_by_class(receiver: &Node, member: Node, methods: &MethodNames, renamed_member: Node) -> Node {
	let name = leading_name(&member);
	let library_call = key(receiver.clone(), Op::Dot, member);
	let method_call = key(receiver.clone(), Op::Dot, renamed_member);
	methods
		.defined_by
		.iter()
		.filter(|(method, _)| *method == name)
		.fold(library_call, |otherwise, (_, class)| by_class(receiver, class, method_call.clone(), otherwise))
}

/// `if receiver is class then method else otherwise`
pub(crate) fn by_class(receiver: &Node, class: &str, method: Node, otherwise: Node) -> Node {
	let bindings = [
		("SELF_WORD", receiver.clone()),
		("CLASS", Node::Symbol(class.to_string())),
		("METHOD", method),
		("OTHERWISE", otherwise),
	];
	super::nodes::filled(DISPATCH_TEMPLATE, bindings)
}

fn method_name(name: &str) -> String {
	format!("{METHOD_PREFIX}{name}")
}

/// The clashing methods renamed where they are defined and called: `c.low`, `c.low()`, and in a class body
/// `low()` and the definition `low() := …`
fn with_method_names(node: Node, methods: &MethodNames, in_class: bool) -> Node {
	let renamed = |name: &Node| match name.drop_meta() {
		Node::Symbol(word) if methods.names.contains(word) => Some(Node::Symbol(method_name(word))),
		_ => None,
	};
	let renamed_call = |call: &Node| match call.drop_meta() {
		Node::List(items, Bracket::Round, separator) if !items.is_empty() => renamed(&items[0])
			.map(|name| Node::List([vec![name], items[1..].to_vec()].concat(), Bracket::Round, separator.clone())),
		_ => None,
	};
	let is_instance = |receiver: &Node| match receiver.drop_meta() {
		Node::Symbol(variable) => methods.instances.contains(variable),
		// `Range(1, 2).low()`, a construction
		Node::List(items, Bracket::Round, _) => {
			items.first().is_some_and(|class| methods.classes.contains(&class.drop_meta().name()))
		}
		Node::Key(class, Op::None | Op::Colon, _) => methods.classes.contains(&class.drop_meta().name()),
		_ => false,
	};
	let recurse = |child: Node| with_method_names(child, methods, in_class);
	match node {
		Node::Type { name, body } => Node::Type { name, body: Box::new(with_method_names(*body, methods, true)) },
		Node::Key(receiver, Op::Dot, member) if is_instance(&receiver) => {
			let member = renamed(&member).or_else(|| renamed_call(&member)).unwrap_or_else(|| recurse(*member));
			key(recurse(*receiver), Op::Dot, member)
		}
		// `x.low` of an x of no known class, outside the classes (whose fields have declared types)
		Node::Key(receiver, Op::Dot, member)
			if !in_class
				&& matches!(receiver.drop_meta(), Node::Symbol(variable) if !methods.instances.contains(variable))
				&& (renamed(&member).is_some() || renamed_call(&member).is_some()) =>
		{
			let member = with_arguments(*member, recurse);
			let renamed_member = renamed(&member).or_else(|| renamed_call(&member)).expect("guarded");
			dispatched_by_class(&receiver, member, methods, renamed_member)
		}
		// `r.low` of an uncertain number: the member keeps its name, its arguments may call methods
		Node::Key(receiver, Op::Dot, member) => key(recurse(*receiver), Op::Dot, with_arguments(*member, recurse)),
		// `low(p)` of an instance is `p.low()` (card one-method-form: x.f() ≡ f(x))
		Node::List(items, Bracket::Round, separator)
			if !in_class && items.len() > 1 && renamed(&items[0]).is_some() && is_instance(&items[1]) =>
		{
			let mut items: Vec<Node> = items.into_iter().map(recurse).collect();
			let receiver = items.remove(1);
			items[0] = renamed(&items[0]).expect("guarded");
			key(receiver, Op::Dot, Node::List(items, Bracket::Round, separator))
		}
		Node::List(..) if in_class && renamed_call(&node).is_some() => {
			let Node::List(items, bracket, separator) = renamed_call(&node).expect("guarded") else {
				unreachable!("a call")
			};
			Node::List(items.into_iter().map(recurse).collect(), bracket, separator)
		}
		// `low() := …`, or a getter `low := …`
		Node::Key(head, Op::Define, body)
			if in_class && (renamed(&head).is_some() || renamed_call(&head).is_some()) =>
		{
			let head = renamed(&head).or_else(|| renamed_call(&head)).expect("guarded");
			key(head, Op::Define, recurse(*body))
		}
		// `low(p:P) := …` outside the class
		Node::Key(head, op @ (Op::Define | Op::Assign), body)
			if !in_class
				&& renamed_call(&head).is_some()
				&& first_parameter(&head).is_some_and(|(_, class)| methods.classes.contains(&class)) =>
		{
			key(renamed_call(&head).expect("guarded"), op, recurse(*body))
		}
		other => other.map_children(recurse),
	}
}

/// The call `name(args)` with each argument passed through `change`, the name kept
fn with_arguments(member: Node, change: impl Fn(Node) -> Node) -> Node {
	match member {
		Node::List(items, Bracket::Round, separator) if !items.is_empty() => Node::List(
			items[..1].iter().cloned().chain(items[1..].iter().cloned().map(change)).collect(),
			Bracket::Round,
			separator,
		),
		other => other,
	}
}

/// Kotlin's `p.copy(y = 5)`: a new instance with p's fields, those changed: `field_with(instance_copy(p, no), "y", 5)`;
/// `shallow = yes` among them asks for the shallow copy (P205)
fn copies(node: Node) -> Node {
	let changed = |argument: &Node| match argument.drop_meta() {
		Node::Key(field, Op::Assign | Op::Colon, value) if matches!(field.drop_meta(), Node::Symbol(_)) => {
			Some((field.drop_meta().name(), value.as_ref().clone()))
		}
		_ => None,
	};
	match node {
		Node::Key(receiver, Op::Dot, member) => {
			let receiver = copies(*receiver);
			let member = copies(*member);
			let changes: Option<Vec<(String, Node)>> = match member.drop_meta() {
				Node::List(items, Bracket::Round, _) if items.len() > 1 && items[0].drop_meta().name() == COPY_WORD => {
					items[1..].iter().map(changed).collect()
				}
				_ => None,
			};
			match changes {
				Some(mut changes) => {
					let shallow = changes
						.iter()
						.position(|(field, _)| field == crate::library_words::SHALLOW)
						.map_or(Node::False, |at| changes.remove(at).1);
					changes.into_iter().fold(
						crate::library_words::copy_call(receiver, shallow),
						|object, (field, value)| {
							let call = vec![symbol(crate::library_words::FIELD_WITH), object, Node::Text(field), value];
							parens(call)
						},
					)
				}
				None => key(receiver, Op::Dot, member),
			}
		}
		other => other.map_children(copies),
	}
}

/// `trait has·area{area}` for each method name that two or more classes define, taking the first one's parameters
fn shared_method_traits(node: &Node) -> Vec<Node> {
	let mut methods: Vec<(String, Node)> = vec![];
	node.visit(&mut |part| {
		let Node::Type { body, .. } = part else { return };
		for item in class_items(body) {
			if let Some((name, parameters)) = instance_member(&item) {
				let receiver = symbol(SELF_WORD);
				let requirement = match parameters.is_empty() {
					true => Node::Symbol(name.clone()),
					false => parens([vec![Node::Symbol(name.clone()), receiver], parameters].concat()),
				};
				methods.push((name, requirement));
			}
		}
	});
	let mut traits = vec![];
	for (index, (name, requirement)) in methods.iter().enumerate() {
		let first = methods.iter().position(|(other, _)| other == name) == Some(index);
		// a library word's methods stay methods of its generic function, so other values keep the library's (traits.rs)
		let shared = methods.iter().filter(|(other, _)| other == name).count() > 1 && !is_library_method(name);
		if first && shared {
			let requirements = Node::List(vec![requirement.clone()], Bracket::Curly, Separator::Space);
			let declaration = key(Node::Symbol(format!("{IMPLICIT_TRAIT_PREFIX}{name}")), Op::Colon, requirements);
			traits.push(Node::List(vec![symbol(TRAIT_KEYWORD), declaration], Bracket::None, Separator::Space));
		}
	}
	traits
}

/// The operation `area` an implicit trait `trait has·area{…}` declares
fn trait_operation(declaration: &Node) -> Option<String> {
	let Node::List(items, _, _) = declaration else {
		return None;
	};
	Some(items.get(1)?.drop_meta().name().strip_prefix(IMPLICIT_TRAIT_PREFIX)?.to_string())
}

/// `x.m` and `x.m(args)` of the methods `m` as calls `m(x)`, `m(x, args)`; of a method changing its object, on a variable,
/// the update `x = m(x, args)`; each call's pair is a temp of its own, `pop·result·1`, so a call inside a method shares
/// no name with one in main. A library word's method on a field of no declared class (`items.pop()` in
/// `pop() := items.pop()`) stays the library word
fn method_calls(node: Node, methods: &MethodCalls, calls: &Counter) -> Node {
	let MethodCalls { called, changing, instance_fields } = methods;
	let recurse = |child: Node| method_calls(child, methods, calls);
	let Node::Key(receiver, Op::Dot, member) = node else {
		return node.map_children(recurse);
	};
	let receiver = recurse(*receiver);
	let (name, arguments) = match member.drop_meta() {
		Node::Symbol(name) => (name.clone(), vec![]),
		Node::List(items, Bracket::Round, _) if !items.is_empty() => {
			(items[0].drop_meta().name(), items[1..].iter().cloned().map(recurse).collect())
		}
		_ => return key(receiver, Op::Dot, recurse(*member)),
	};
	let library_call_on_field = is_library_method(&name)
		&& matches!(receiver.drop_meta(), Node::Key(_, Op::Dot, field) if !instance_fields.contains(&field.drop_meta().name()));
	if library_call_on_field || !called.contains(&name) && !changing.contains(&name) {
		return key(receiver, Op::Dot, recurse(*member));
	}
	let call = parens([Node::Symbol(name.clone()), receiver.clone()].into_iter().chain(arguments).collect());
	let item = |pair: Node, position: i64| key(pair, Op::Hash, Node::int(position));
	match receiver.drop_meta() {
		// `s.pop()`: the pair (value, changed object) of the call, the object stored back, the value given
		Node::Symbol(_) if changing.giving_value.contains(&name) => {
			let pair = Node::Symbol(format!("{name}{RESULT_SUFFIX}·{}", calls.next_number()));
			Node::List(
				vec![assign(pair.clone(), call), assign(receiver, item(pair.clone(), 2)), item(pair, 1)],
				Bracket::Round,
				Separator::Semicolon,
			)
		}
		_ if changing.giving_value.contains(&name) => item(call, 1),
		Node::Symbol(_) if changing.itself.contains(&name) => assign(receiver, call),
		_ => call,
	}
}

/// What method_calls rewrites: the shared methods, the changing ones, and the fields that may hold an instance
struct MethodCalls<'a> {
	called: &'a [String],
	changing: &'a Changing,
	instance_fields: &'a [String],
}

/// The fields some class declares with a class type (`top: Node`, not `items: list`, `[Point]` or `items = []`)
fn class_typed_fields(node: &Node) -> Vec<String> {
	let mut fields = vec![];
	node.visit(&mut |part| {
		if let Node::Type { body, .. } = part {
			for item in class_items(body).iter().filter(|item| method_parts(item).is_none()) {
				let type_name = field_type_name(item);
				let is_class = !type_name.is_empty()
					&& !type_name.starts_with('[')
					&& crate::type_tests::builtin_type_name(&type_name).is_none();
				fields.extend(declared_field_name(item).filter(|_| is_class));
			}
		}
	});
	fields
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
	let Node::Type { name, body } = node.drop_meta() else {
		return None;
	};
	let class = name.drop_meta().name();
	let (statics, items): (Vec<Node>, Vec<Node>) = class_items(body).into_iter().partition(is_static);
	let (constructors, items): (Vec<Node>, Vec<Node>) =
		items.into_iter().partition(|item| constructor_body(item).is_some());
	let (methods, mut fields): (Vec<Node>, Vec<Node>) =
		items.into_iter().partition(|item| method_parts(item).is_some());
	if methods.is_empty() && statics.is_empty() && constructors.is_empty() {
		return None;
	}
	let mut field_names: Vec<String> = fields.iter().filter_map(declared_field_name).collect();
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
	let siblings: Vec<(String, bool)> =
		methods.iter().filter_map(method_parts).map(|(name, parameters, _)| (name, parameters.is_empty())).collect();
	let members = Members { class: &class, fields: &field_names, methods: &siblings, statics: &static_names };
	let mut functions: Vec<Node> = statics.iter().flat_map(|member| static_definitions(&members, member)).collect();
	// the constructor gives the instance it was handed, its fields set
	let gives_itself =
		|body: &Node| Node::List(vec![body.clone(), symbol(SELF_WORD)], Bracket::None, Separator::Semicolon);
	functions.extend(
		constructors
			.iter()
			.filter_map(constructor_parts)
			.map(|(parameters, body)| function(&members, &constructor_name(&class), parameters, gives_itself(body)).0),
	);
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

/// `greet(x) := body` or `area := body` in a class body: name, parameters and body
fn method_parts(item: &Node) -> Option<(String, Vec<Node>, Node)> {
	let Node::Key(head, Op::Define, body) = item.drop_meta() else {
		return None;
	};
	let head = match head.drop_meta() {
		Node::Key(untyped, Op::Colon, _) if result_type(item).is_some() => untyped,
		_ => head,
	};
	match head.drop_meta() {
		Node::Symbol(name) => Some((name.clone(), vec![], *body.clone())),
		Node::List(parts, Bracket::Round, _) => match parts.split_first() {
			Some((name, parameters)) if matches!(name.drop_meta(), Node::Symbol(_)) => {
				Some((name.drop_meta().name(), parameters.to_vec(), *body.clone()))
			}
			_ => None,
		},
		_ => None,
	}
}

/// `name`, `age:int`, `left?`, `x:int=0`, `k = 3`: the name of a field
fn declared_field_name(item: &Node) -> Option<String> {
	match item.drop_meta() {
		Node::Symbol(name) => Some(name.trim_end_matches(FIELD_MARKS).to_string()),
		Node::Key(field, Op::Colon, _) => Some(field.drop_meta().name().trim_end_matches(FIELD_MARKS).to_string()),
		Node::Key(field, Op::Assign, _) => declared_field_name(field),
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
	let parameter_names: Vec<String> = parameters
		.iter()
		.map(|parameter| match parameter.drop_meta() {
			Node::Key(name, Op::Colon, _) => name.drop_meta().name(),
			other => other.name(),
		})
		.collect();
	let unshadowed = |name: &String| !parameter_names.contains(name);
	let fields: Vec<&String> = members.fields.iter().filter(|field| unshadowed(field)).collect();
	let methods: Vec<&String> = members.methods.iter().map(|(name, _)| name).filter(|name| unshadowed(name)).collect();
	let getters: Vec<&String> = members
		.methods
		.iter()
		.filter(|(_, getter)| *getter)
		.map(|(name, _)| name)
		.filter(|name| unshadowed(name))
		.collect();
	let statics: Vec<&String> = members.statics.iter().filter(|name| unshadowed(name)).collect();
	let readable = Readable { receiver: SELF_WORD, class, fields: [fields, getters].concat(), methods, statics };
	// the holes of `"#\(n)"` as code first, so a field in one is read from self too
	let body = crate::text::interpolation::lower_program(body);
	let body = receiver_reads(body, &readable);
	let receiver = symbol(SELF_WORD);
	let changes = match changes_fields_of(&body, SELF_WORD) {
		false => Change::None,
		true if gives_value(&body) => Change::GivingValue,
		true => Change::Itself,
	};
	let body = match changes {
		Change::None => body,
		Change::Itself => {
			Node::List(vec![with_returns(body, &|_| receiver.clone()), receiver], Bracket::None, Separator::Semicolon)
		}
		Change::GivingValue => {
			// an early `return v` gives the pair too: `if n <= 0 { return 0 }; n -= 1; n`
			let paired = |value: Node| Node::List(vec![value, receiver.clone()], Bracket::Square, Separator::Space);
			// the statements before the value run first: `x = items#1; items = …; x` gives x, not the block
			let mut statements = with_returns(body, &paired).body_statements();
			let value = statements.pop().unwrap_or(Node::Empty);
			let result = Node::Symbol(format!("{method}{VALUE_SUFFIX}"));
			let pair = Node::List(vec![result.clone(), receiver], Bracket::Square, Separator::Space);
			statements.extend([assign(result, value), pair]);
			Node::List(statements, Bracket::None, Separator::Semicolon)
		}
	};
	let receiver = key(symbol(SELF_WORD), Op::Colon, symbol(class));
	let head = parens([vec![symbol(method), receiver], parameters].concat());
	(key(head, Op::Define, body), changes)
}

/// Each `return v` of the body (not of a function or lambda inside it) as `return given(v)`
fn with_returns(node: Node, given: &dyn Fn(Node) -> Node) -> Node {
	let is_return = |word: &Node| word.is_symbol(RETURN_WORD);
	match node {
		Node::Symbol(_) if is_return(&node) => {
			Node::List(vec![node, given(Node::Empty)], Bracket::None, Separator::Space)
		}
		Node::List(items, bracket, separator) if items.len() <= 2 && items.first().is_some_and(is_return) => {
			let value = items.get(1).cloned().unwrap_or(Node::Empty);
			Node::List(vec![items[0].clone(), given(value)], bracket, separator)
		}
		Node::Key(_, Op::Define | Op::Arrow, _) => node,
		other => other.map_children(|child| with_returns(child, given)),
	}
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
	let Some(last) = body.body_statements().pop() else {
		return false;
	};
	match last.drop_meta() {
		// `if c {items.add(x)}`: a branch gives what its last statement gives
		Node::Key(_, Op::Then, branch) => gives_value(branch),
		Node::Key(then, Op::Else, otherwise) => gives_value(then) || gives_value(otherwise),
		// a nested sequence gives what its last statement gives
		Node::List(_, Bracket::Round, Separator::Semicolon) | Node::List(_, Bracket::Curly, _) => gives_value(&last),
		Node::Key(_, op, _) if *op == Op::Define || op.is_write() => false,
		// a body giving its object back already (a constructor)
		Node::Symbol(name) if name == SELF_WORD => false,
		Node::Key(_, Op::Dot, call) => {
			!mutating_call(call)
				|| matches!(call.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|word| GIVING_MUTATIONS.contains(&word.drop_meta().name().as_str())))
		}
		_ => true,
	}
}

/// The names a method body reads from its receiver: fields and getters (`area`), and methods it calls (`area()`)
#[derive(Default, Clone)]
struct Readable<'a> {
	/// `self` in a class body, the first parameter of an outside method
	receiver: &'a str,
	class: &'a str,
	fields: Vec<&'a String>,
	methods: Vec<&'a String>,
	statics: Vec<&'a String>,
}

/// The body with each field name and getter read from self, each call of a method of the class a call on self (as in
/// Java), `this` as self; the member name right of a dot stays as it is
fn receiver_reads(node: Node, readable: &Readable) -> Node {
	let on_receiver = |member: Node| key(symbol(readable.receiver), Op::Dot, member);
	let is_call_of = |items: &[Node], names: &[&String]| {
		items.first().and_then(Node::symbol_name).is_some_and(|name| names.iter().any(|known| *known == name))
	};
	match node {
		Node::Symbol(name) if readable.statics.contains(&&name) => Node::Symbol(static_name(readable.class, &name)),
		Node::Symbol(name) if readable.fields.contains(&&name) => on_receiver(Node::Symbol(name)),
		Node::Symbol(name) if RECEIVER_ALIASES.contains(&name.as_str()) => symbol(readable.receiver),
		Node::List(items, Bracket::Round, separator) if is_call_of(&items, &readable.methods) => {
			on_receiver(arguments_rewritten(items, Bracket::Round, separator, |argument| {
				receiver_reads(argument, readable)
			}))
		}
		Node::Key(object, Op::Dot, member) => Node::Key(Box::new(receiver_reads(*object, readable)), Op::Dot, member),
		Node::List(items, bracket, separator) if separator.separates_statements() => {
			Node::List(statements_reads(items, readable), bracket, separator)
		}
		other if let_binding(&other).is_some() => other.map_children(|part| value_reads(part, readable)),
		other => other.map_children(|child| receiver_reads(child, readable)),
	}
}

/// `n = value` of `let n = value`: the value reads the receiver, n is the new local
fn value_reads(binding: Node, readable: &Readable) -> Node {
	match binding {
		Node::Key(target, Op::Assign, value) => {
			Node::Key(target, Op::Assign, Box::new(receiver_reads(*value, readable)))
		}
		Node::Meta { node, data } => Node::Meta { node: Box::new(value_reads(*node, readable)), data },
		word => word,
	}
}

/// Statements in order: from `let n = …` on, n is the method's local, no longer its receiver's field
fn statements_reads(statements: Vec<Node>, readable: &Readable) -> Vec<Node> {
	let mut readable = readable.clone();
	statements
		.into_iter()
		.map(|statement| {
			let read = receiver_reads(statement, &readable);
			if let Some(local) = let_binding(&read) {
				readable.fields.retain(|field| field.as_str() != local);
			}
			read
		})
		.collect()
}

/// The name `let name = …` binds
fn let_binding(statement: &Node) -> Option<String> {
	let Node::List(items, _, Separator::Space) = statement.drop_meta() else {
		return None;
	};
	match items.as_slice() {
		[keyword, binding] if keyword.is_symbol(crate::words::LET_WORD) => match binding.drop_meta() {
			Node::Key(target, Op::Assign, _) => target.drop_meta().symbol_name().map(str::to_string),
			_ => None,
		},
		_ => None,
	}
}

/// Does the body assign a field of the variable (`self.n = …`, `self.n += 1`, `self.n++`) or change a list in one
/// (`self.items.add(x)`)
fn changes_fields_of(body: &Node, variable: &str) -> bool {
	any_part(body, |part| match part {
		Node::Key(target, Op::Dot, call) if is_field_of(target, variable) => mutating_call(call),
		Node::Key(target, op, _) => op.is_write() && is_field_of(target, variable),
		_ => false,
	})
}

/// `self.items`, `self.stack.items`, an element `self.counts#i`
fn is_field_of(target: &Node, variable: &str) -> bool {
	match target.drop_meta() {
		Node::Key(object, Op::Dot, _) => {
			matches!(object.drop_meta(), Node::Symbol(name) if name == variable) || is_field_of(object, variable)
		}
		Node::Key(list, Op::Hash, _) => is_field_of(list, variable),
		_ => false,
	}
}

/// `add(x)`, `insert(x, at:1)`: a call of a method that changes the list it is called on
fn mutating_call(call: &Node) -> bool {
	matches!(call.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().and_then(Node::symbol_name).is_some_and(crate::list_mutations::is_list_mutating_method))
}
