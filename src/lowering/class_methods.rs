//! Methods in a class body (issue #14, notes/classes.md): `class person{name; greet() := "hi " + name}` is the class
//! `person{name}` and the function `greet(self:person) := "hi " + self.name`. In a method body a bare field name and
//! `self`/`this` read the receiver; `p.greet()`, `greet(p)` and, for a method without parameters, `p.area` call it.
//! A method name that several classes define is declared as an implicit trait, so a call picks the class's own method
//! by the static type of its receiver (traits.rs witnesses).
//! `class dog extends animal {…}` (P117) gives dog the fields and methods of animal, its own ones override them, and
//! declares `dog like animal`, so a dog is accepted where an animal is wanted.

use super::words::{GLOBAL_WORD, MAP_WORD, RETURN_WORD, SUM_WORD};
use super::nodes::{Counter, call, grouped_parameters, if_then, key};
mod inheritance;
mod operators;
mod other_languages;
use inheritance::*;
use operators::*;
use other_languages::*;
use crate::node::{symbol, text, Bracket, Node, Separator};
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
/// Other languages' method names (Python's deque, Java's Deque and Queue, JS's Array and Set) and the warp methods they
/// mean, the first one the class defines taken, with a note; only on a class that does not define the name itself
const METHOD_ALIASES: [(&str, &[&str]); 23] = [
	("clone", &["copy"]), // clone always calls the class's copy (P207)
	("append", &["push_back", "push", "enqueue", "add"]), ("addLast", &["push_back", "enqueue"]), ("offerLast", &["push_back", "enqueue"]),
	("offer", &["enqueue", "push_back"]), ("push", &["push_back", "enqueue"]),
	("appendleft", &["push_front"]), ("addFirst", &["push_front"]), ("offerFirst", &["push_front"]), ("unshift", &["push_front"]),
	("popleft", &["pop_front", "dequeue"]), ("pollFirst", &["pop_front", "dequeue"]), ("removeFirst", &["pop_front", "dequeue"]),
	("shift", &["pop_front", "dequeue"]), ("poll", &["dequeue", "pop_front"]),
	("pollLast", &["pop_back", "pop"]), ("removeLast", &["pop_back", "pop"]), ("pop", &["pop_back"]),
	("contains", &["has"]), ("includes", &["has"]), ("delete", &["remove"]), ("discard", &["remove"]),
	("len", &["size"]),
];
/// Member modifiers that may mean something in warp, so they get no note that warp needs them not
const SILENT_MODIFIERS: [&str; 1] = ["async"];
/// The constructor names of other languages, aliases of `init` (P162): warp's old `value`, JavaScript, Python, Ruby,
/// PHP, VB.NET, Delphi, Rust; besides a method named like its class (C++, Java, C#)
const CONSTRUCTOR_ALIASES: [&str; 8] = ["value", "constructor", "__init__", "initialize", "__construct", "New", "Create", "new"];
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
/// Kotlin's `p.copy(y = 5)`
const COPY_WORD: &str = "copy";
/// lib/units.warp's `q.to("km/h")`: a run-time quantity in other units
const CONVERSION_METHOD: &str = "to";
/// lib/units.warp's check that two quantities measure the same, and a quantity's amount in base units
const SAME_DIMENSION: &str = "same_dimension";
const QUANTITY_AMOUNT: &str = "amount";
/// The sum of a list of run-time quantities folds their `plus` (quantities_reduced)
const REDUCE_WORD: &str = "reduce";
const REDUCED_NAMES: [&str; 2] = ["quantity·sum", "quantity·item"];
/// `str(q)`: a run-time quantity's text
const TEXT_WORD: &str = "str";
/// The methods an operator on an instance calls (wiki/operator.md aliases, Python's special methods)
const OPERATOR_METHODS: [(Op, &[&str]); 9] = [
	(Op::Add, &["plus", "add", "__add__"]),
	(Op::Sub, &["minus", "subtract", "__sub__"]),
	(Op::Mul, &["times", "multiply", "__mul__"]),
	(Op::Div, &["divide", "div", "__truediv__"]),
	(Op::Mod, &["mod", "modulo", "__mod__"]),
	(Op::Lt, &["less", "smaller", "__lt__"]),
	(Op::Gt, &["more", "bigger", "__gt__"]),
	(Op::Similar, &["approximately"]),
	(Op::Rough, &["similar"]),
];
/// P212: a class defining the method of only one of these operators has it serve the other too
const INTERCHANGEABLE_OPERATORS: [(Op, Op); 2] = [(Op::Similar, Op::Rough), (Op::Rough, Op::Similar)];
/// The run-time choice of a library-word method by the receiver's class (dispatched_by_class)
const DISPATCH_TEMPLATE: &str = "if RECEIVER is CLASS then METHOD else OTHERWISE";
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
	let node = if shared.is_empty() && changing.is_empty() { node } else { method_calls(node, &shared, &changing, &Counter::starting_at(1)) };
	match (traits.is_empty(), node) {
		(true, node) => node,
		(false, Node::List(items, bracket, separator)) if separator != Separator::Space => Node::List([traits, items].concat(), bracket, separator),
		(false, node) => Node::List([traits, vec![node]].concat(), Bracket::None, Separator::Semicolon),
	}
}

/// `d.append(1)` of a deque, `s.contains(2)` of a set, `len(s)`: the class's own method when it does not define that
/// name (METHOD_ALIASES, analyzer::COUNTING_WORDS), with a note
fn method_aliases(node: Node) -> Node {
	let mut methods: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		methods.insert(name.drop_meta().name(), class_items(body).iter().filter_map(method_parts).map(|(method, _, _)| method).collect());
	});
	if methods.is_empty() {
		return node;
	}
	let classes: Vec<String> = methods.keys().cloned().collect();
	let instances: std::collections::HashMap<String, Vec<String>> = instance_classes(&node, &classes).into_iter().map(|(variable, class)| (variable, methods[&class].clone())).collect();
	with_method_aliases(node, &instances)
}

/// The method of the class for `written`: the name itself, or the alias's warp method, or for a size word the size
/// method the class defines
fn aliased_method(written: &str, defined: &[String]) -> Option<String> {
	if defined.iter().any(|method| method == written) {
		return None;
	}
	let candidates: Vec<&str> = match crate::analyzer::is_counting_word(written) {
		true => crate::analyzer::COUNTING_WORDS.to_vec(),
		false => METHOD_ALIASES.iter().find(|(alias, _)| *alias == written).map(|(_, methods)| methods.to_vec()).unwrap_or_default(),
	};
	let method = candidates.into_iter().find(|candidate| defined.iter().any(|method| method == candidate))?;
	crate::diagnostic::note_alias(written, method);
	Some(method.to_string())
}

fn with_method_aliases(node: Node, instances: &std::collections::HashMap<String, Vec<String>>) -> Node {
	let defined_by = |receiver: &Node| match receiver.drop_meta() {
		Node::Symbol(variable) => instances.get(variable),
		_ => None,
	};
	match node {
		Node::Key(receiver, Op::Dot, member) if defined_by(&receiver).is_some() => {
			let member = match member.drop_meta() {
				Node::List(items, Bracket::Round, separator) if !items.is_empty() => match aliased_method(&items[0].drop_meta().name(), defined_by(&receiver).expect("guarded")) {
					Some(method) => Node::List([vec![Node::Symbol(method)], items[1..].iter().cloned().map(|item| with_method_aliases(item, instances)).collect()].concat(), Bracket::Round, separator.clone()),
					None => with_method_aliases(*member, instances),
				},
				_ => *member,
			};
			Node::Key(receiver, Op::Dot, Box::new(member))
		}
		// `len(s)`: `s.size()`; `size(s)` of a class defining size(): `s.size()` too (card size-instance)
		Node::List(items, Bracket::Round, separator) if matches!(items.as_slice(), [word, argument] if crate::analyzer::is_counting_word(&word.drop_meta().name()) && defined_by(argument).is_some()) => {
			let written = items[0].drop_meta().name();
			let defined = defined_by(&items[1]).expect("guarded");
			let method = match defined.contains(&written) {
				true => Some(written),
				false => aliased_method(&written, defined),
			};
			match method {
				Some(method) => key(items[1].clone(), Op::Dot, call(&method, vec![])),
				None => Node::List(items, Bracket::Round, separator),
			}
		}
		other => other.map_children(|child| with_method_aliases(child, instances)),
	}
}

/// json's word for a program with classes (lib/json.warp): the classes' names go along, so an instance is its fields
const TO_JSON_WORD: &str = "to_json";
const TO_JSON_OF_CLASSES_WORD: &str = "to_json_of_classes";
/// Other languages' calls giving an instance's json or its fields: Kotlin's `Json.encodeToString(p)` is to_json(p),
/// Python's `dataclasses.asdict(p)` the instance itself (its fields read like a map's entries)
const SERIALIZER_ALIASES: [(&str, &str, &str); 2] = [("Json", "encodeToString", TO_JSON_WORD), ("dataclasses", "asdict", "")];

/// `to_json(p)` in a program declaring classes: `to_json_of_classes(p, ["Point", …])`, after std_aliases made
/// `JSON.stringify(p)` and `json.dumps(p)` to_json
pub fn lower_json_classes(node: Node) -> Node {
	let mut classes = vec![];
	node.visit(&mut |part| if let Node::Type { name, .. } = part {
		classes.push(Node::Text(name.drop_meta().name()));
	});
	if classes.is_empty() {
		return node;
	}
	with_json_classes(node, &Node::List(classes, Bracket::Square, Separator::Space))
}

fn with_json_classes(node: Node, classes: &Node) -> Node {
	let serializer = |module: &Node, member: &Node| SERIALIZER_ALIASES.iter().find(|(written_module, written_member, _)| module.drop_meta().name() == *written_module && leading_name(member) == *written_member);
	match node {
		Node::Key(module, Op::Dot, member) if serializer(&module, &member).is_some() => {
			let (written_module, written_member, word) = serializer(&module, &member).expect("guarded");
			let Node::List(items, Bracket::Round, _) = member.drop_meta() else { return Node::Key(module, Op::Dot, member) };
			let argument = items.get(1).cloned().unwrap_or(Node::Empty);
			crate::diagnostic::note_alias(&format!("{written_module}.{written_member}"), if word.is_empty() { "the instance" } else { word });
			match word.is_empty() {
				true => with_json_classes(argument, classes),
				false => with_json_classes(call(word, vec![argument]), classes),
			}
		}
		Node::List(items, Bracket::Round, separator) if items.len() == 2 && items[0].drop_meta().name() == TO_JSON_WORD => {
			let value = with_json_classes(items[1].clone(), classes);
			Node::List(vec![symbol(TO_JSON_OF_CLASSES_WORD), value, classes.clone()], Bracket::Round, separator)
		}
		other => other.map_children(|child| with_json_classes(child, classes)),
	}
}

/// A class component's method giving its markup (Lit, React classes); other frameworks' names for it, with a note:
/// Backbone's and Mithril's `view`, Angular's `template`, Flutter's `build`
const RENDER_WORD: &str = "render";
const RENDER_ALIASES: [&str; 3] = ["view", "template", "build"];

/// Class components (notes/web_framework.md step 4): an instance of a class with a render() method is its markup where
/// markup is expected, a child of an element (`div{ Greeting("Ann") }`) or the program's value; anywhere else it stays
/// an instance
fn rendered_components(node: Node) -> Node {
	let node = with_render_aliases(node);
	let mut components = vec![];
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		if class_items(body).iter().filter_map(method_parts).any(|(method, _, _)| method == RENDER_WORD) {
			components.push(name.drop_meta().name());
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
fn with_render_aliases(node: Node) -> Node {
	match node {
		Node::Type { name, body } => {
			let items = class_items(&body);
			let methods: Vec<String> = items.iter().filter_map(method_parts).map(|(method, _, _)| method).collect();
			let alias = RENDER_ALIASES.iter().find(|alias| methods.iter().any(|method| method == *alias));
			match alias.filter(|_| !methods.iter().any(|method| method == RENDER_WORD)) {
				Some(alias) => {
					crate::diagnostic::note_alias(alias, RENDER_WORD);
					let items = items.into_iter().map(|item| match method_parts(&item) {
						Some((method, _, _)) if method == *alias => renamed(&item, RENDER_WORD),
						_ => item,
					}).collect();
					Node::Type { name, body: Box::new(Node::List(items, Bracket::Curly, Separator::Semicolon)) }
				}
				None => Node::Type { name, body },
			}
		}
		other => other.map_children(with_render_aliases),
	}
}

/// The children of each element rendered
fn with_rendered_children(node: Node, components: &[String]) -> Node {
	match node {
		Node::Key(tag, op @ (Op::None | Op::Colon), children) if crate::markup::is_element_tag(&tag.drop_meta().name()) && matches!(children.drop_meta(), Node::List(_, Bracket::Curly, _)) => {
			let Node::List(items, bracket, separator) = children.drop_meta().clone() else { unreachable!("guarded") };
			let items = items.into_iter().map(|item| rendered(with_rendered_children(item, components), components)).collect();
			Node::Key(tag, op, Box::new(Node::List(items, bracket, separator)))
		}
		other => other.map_children(|child| with_rendered_children(child, components)),
	}
}

/// A construction of a component as `Greeting("Ann").render()`
fn rendered(node: Node, components: &[String]) -> Node {
	let is_construction = matches!(node.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().is_some_and(|class| components.contains(&class.drop_meta().name())));
	match is_construction {
		true => key(node, Op::Dot, call(RENDER_WORD, vec![])),
		false => node,
	}
}

/// `Point.from_json(t)` reads the json, as `parse_json(t) as Point` (std json)
const FROM_JSON_WORD: &str = "from_json";
const PARSE_JSON_WORD: &str = "parse_json";
/// The variable holding the object an instance is built from: `Point·from`
const FROM_SUFFIX: &str = "·from";
/// The element of a list of instances built from objects: `Point·element`
const ELEMENT_SUFFIX: &str = "·element";
/// The variable of a list of instances built before its construction: `elements·1`
const ELEMENTS_WORD: &str = "elements";

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
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		let typed = class_items(body).iter().filter(|item| method_parts(item).is_none()).filter_map(|item| Some((field_name(item)?, field_type_name(item)))).collect();
		fields.insert(name.drop_meta().name(), typed);
	});
	fields
}

/// Each field of a class body: its name, type name (empty when untyped) and default value (`legs: int = 4`)
pub(crate) fn field_declarations(body: &Node) -> Vec<(String, String, Option<Node>)> {
	class_items(body).iter().filter(|item| method_parts(item).is_none()).filter_map(|item| {
		let default = match item.drop_meta() {
			Node::Key(_, Op::Assign, value) => Some(value.as_ref().clone()),
			_ => None,
		};
		Some((field_name(item)?, field_type_name(item), default))
	}).collect()
}

/// The fields annotated `@attribute(value)` (`@was(nick) name: text`): each name and its value
pub(crate) fn fields_marked(body: &Node, attribute: &str) -> Vec<(String, Node)> {
	class_items(body).iter().filter_map(|item| Some((field_name(item)?, field_attribute(item, attribute)?.clone()))).collect()
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
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		let parent = name.attribute(crate::warp_parser::EXTENDS_KEYWORD).map(|parent| parent.drop_meta().name());
		classes.insert(name.drop_meta().name(), (parent, class_items(body).iter().filter_map(item_name).collect()));
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
	let mut layouts = std::collections::HashMap::new();
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		let parent = name.attribute(crate::warp_parser::EXTENDS_KEYWORD).map(|parent| parent.drop_meta().name());
		let class = name.drop_meta().name();
		let (mut fields, mut methods) = (vec![], vec![]);
		for item in class_items(body) {
			match method_parts(&item) {
				Some((method, _, _)) if method != CONSTRUCTOR_WORD && method != class && !method.contains('·') => methods.push(method),
				Some(_) => {}
				None => fields.extend(field_name(&item)),
			}
		}
		layouts.insert(class, ClassLayout { parent, fields, methods });
	});
	layouts
}

/// The variable holding what is taken apart: `{x, y} = p` is `(parts·from = p; x = parts·from.x; y = parts·from.y)`
const PARTS_WORD: &str = "parts·from";

/// Destructuring (JS, Python, Rust, Kotlin): `{x, y} = p` and `{x: a} = p` by field name (maps too); `(a, b) = p` and
/// `a, b = p` of an instance in field order; match arms `Point{x, y} => …` and `Point(x, y) => …` a type test binding
/// the fields
fn destructurings(node: Node) -> Node {
	with_value_classes(node, taken_apart)
}

/// P179: a field by its position, `c#1` 1-based and Rust's `c.0` 0-based, of an instance whose class is known
/// (`c = rgb(1, 2, 3)`, `Some(4)#1`): the field by name, `c.value1`
fn positional_fields(node: Node) -> Node {
	with_value_classes(node, by_position)
}

/// `lower` of the program with each class's fields and the class of a value: a variable's instance class or the class
/// a construction makes
fn with_value_classes(node: Node, lower: fn(Node, &ClassFields, &ClassOf) -> Node) -> Node {
	let fields = class_fields(&node);
	let classes: Vec<String> = fields.keys().cloned().collect();
	let instances = instance_classes(&node, &classes);
	let class_of = |value: &Node| match value.drop_meta() {
		Node::Symbol(variable) => instances.get(variable).cloned(),
		other => constructed_class(other).filter(|class| fields.contains_key(class)),
	};
	lower(node, &fields, &class_of)
}

fn by_position(node: Node, fields: &ClassFields, class_of: &ClassOf) -> Node {
	let position = |op: Op, index: &Node| match (op, index.drop_meta()) {
		(Op::Hash, Node::Number(crate::Number::Int(number))) => Some(number - 1),
		(Op::Dot, Node::Number(crate::Number::Int(number))) => Some(*number),
		_ => None,
	};
	// `a: int = Some(3)`: a variable of a built-in type takes the single field of the construction it is given
	let unwrapped_field = |declared: &Node, value: &Node| match (declared.drop_meta(), constructed_class(value).and_then(|class| fields.get(&class))) {
		(Node::Key(_, Op::Colon, kind), Some(class_fields)) if class_fields.len() == 1 && crate::analyzer::builtin_type_kind(&kind.drop_meta().name()).is_some() => Some(class_fields[0].0.clone()),
		_ => None,
	};
	match node {
		Node::Key(declared, Op::Assign, value) if unwrapped_field(&declared, &value).is_some() => {
			let field = unwrapped_field(&declared, &value).expect("guarded");
			Node::Key(declared, Op::Assign, Box::new(field_of(&by_position(*value, fields, class_of), &field)))
		}
		Node::Key(instance, op @ (Op::Hash | Op::Dot), index) if position(op, &index).is_some() && class_of(&instance).is_some() => {
			let class = class_of(&instance).expect("guarded");
			let class_fields = &fields[&class];
			let instance = by_position(*instance, fields, class_of);
			match usize::try_from(position(op, &index).expect("guarded")).ok().and_then(|at| class_fields.get(at)) {
				Some((field, _)) => field_of(&instance, field),
				None => crate::node::error(&format!("{} has {} fields: {} is out of range", class, class_fields.len(), index.serialize().trim())),
			}
		}
		other => other.map_children(|child| by_position(child, fields, class_of)),
	}
}

fn taken_apart(node: Node, fields: &ClassFields, class_of: &ClassOf) -> Node {
	let recurse = |child: Node| taken_apart(child, fields, class_of);
	let subject = symbol(PARTS_WORD);
	match node {
		// `{x, start: Point{x: a}} = l`: names only, no constant to test (a class in it is no test either)
		Node::Key(target, Op::Assign, value) if matched_fields(&target, &subject, fields).is_some_and(|matched| matched.tests.is_empty()) => {
			let matched = matched_fields(&target, &subject, fields).expect("guarded");
			bound_parts(recurse(*value), &matched.bindings, None)
		}
		Node::Key(target, Op::Assign, value) if positional_names(&target).is_some() && class_of(&value).is_some() => {
			let names = positional_names(&target).expect("guarded");
			let class = class_of(&value).expect("guarded");
			if fields[&class].len() != names.len() {
				return Node::Key(target, Op::Assign, value);
			}
			let bindings = fields[&class].iter().zip(names).map(|((field, _), name)| (field_of(&subject, field), name)).collect::<Vec<_>>();
			bound_parts(recurse(*value), &bindings, None)
		}
		// Python's `a, b = p`: the parser reads `a, (b = p)`
		Node::List(items, bracket, separator) if separator == Separator::Colon && last_assigns_instance(&items, class_of) => {
			let Some(Node::Key(last, Op::Assign, value)) = items.last().map(|item| item.drop_meta().clone()) else { return Node::List(items, bracket, separator) };
			let names = [&items[..items.len() - 1], &[*last]].concat();
			let target = Node::List(names, Bracket::Round, Separator::Colon);
			recurse(Node::Key(Box::new(target), Op::Assign, value))
		}
		Node::Key(pattern, Op::FatArrow, body) if is_class_pattern(&pattern, fields) => {
			let mut matched = Matched::default();
			if matched_pattern(&pattern, &subject, fields, &mut matched).is_none() {
				return Node::Key(pattern, Op::FatArrow, Box::new(recurse(*body)));
			}
			// `parts·from if parts·from is Point and parts·from.x == 0` (`is` compares like ==, a class name tests the type)
			let test = matched.class_tests.into_iter().chain(matched.tests).reduce(|all, test| key(all, Op::And, test)).expect("a class pattern tests its class");
			let guard = if_then(test, subject.clone());
			key(guard, Op::FatArrow, bound_parts(subject, &matched.bindings, Some(recurse(*body))))
		}
		other => other.map_children(recurse),
	}
}

/// What a pattern asks of its subject: class tests (`p is Point`), value tests (`p.x == 0`) and the variables it
/// binds, each with its path
#[derive(Default)]
struct Matched {
	class_tests: Vec<Node>,
	tests: Vec<Node>,
	bindings: Vec<(Node, String)>,
}

fn field_of(path: &Node, field: &str) -> Node {
	key(path.clone(), Op::Dot, symbol(field))
}

/// `Point{…}` or `Point(…)` of a declared class
fn is_class_pattern(pattern: &Node, fields: &ClassFields) -> bool {
	match pattern.drop_meta() {
		Node::Key(class, Op::Colon | Op::None, parts) => fields.contains_key(&class.drop_meta().name()) && matches!(parts.drop_meta(), Node::List(_, Bracket::Curly, _)),
		Node::List(items, Bracket::Round, _) => items.len() > 1 && fields.contains_key(&items[0].drop_meta().name()),
		_ => false,
	}
}

/// A pattern at `path`: `_` matches anything, a name binds, a number or text is compared, `Point{x, y: b}` and
/// `Point(a, 0)` test the class and match each field; None for anything else (the arm stays as written)
fn matched_pattern(pattern: &Node, path: &Node, fields: &ClassFields, matched: &mut Matched) -> Option<()> {
	let class_test = |class: &str| key(path.clone(), Op::Eq, symbol(class));
	match pattern.drop_meta() {
		Node::Symbol(name) if name == "_" => {}
		Node::Symbol(name) if !fields.contains_key(name) => matched.bindings.push((path.clone(), name.clone())),
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False => matched.tests.push(key(path.clone(), Op::Eq, pattern.drop_meta().clone())),
		Node::Key(class, Op::Colon | Op::None, parts) if fields.contains_key(&class.drop_meta().name()) => {
			matched.class_tests.push(class_test(&class.drop_meta().name()));
			let inner = matched_fields(parts, path, fields)?;
			matched.class_tests.extend(inner.class_tests);
			matched.tests.extend(inner.tests);
			matched.bindings.extend(inner.bindings);
		}
		Node::List(items, Bracket::Round, _) if items.len() > 1 && fields.contains_key(&items[0].drop_meta().name()) => {
			let class = items[0].drop_meta().name();
			if items.len() - 1 != fields[&class].len() {
				return None;
			}
			matched.class_tests.push(class_test(&class));
			for ((field, _), part) in fields[&class].iter().zip(&items[1..]) {
				matched_pattern(part, &field_of(path, field), fields, matched)?;
			}
		}
		_ => return None,
	}
	Some(())
}

/// `{x, y: b, from: Point{…}}` at `path`: each field by name, a field with a pattern after its colon matched by it
fn matched_fields(parts: &Node, path: &Node, fields: &ClassFields) -> Option<Matched> {
	let Node::List(items, Bracket::Curly, _) = parts.drop_meta() else { return None };
	if items.is_empty() {
		return None;
	}
	let mut matched = Matched::default();
	for item in items {
		match item.drop_meta() {
			Node::Symbol(name) => matched.bindings.push((field_of(path, name), name.clone())),
			Node::Key(field, Op::Colon, part) if matches!(field.drop_meta(), Node::Symbol(_)) => matched_pattern(part, &field_of(path, &field.drop_meta().name()), fields, &mut matched)?,
			_ => return None,
		}
	}
	Some(matched)
}

/// `(parts·from = value; x = parts·from.x; …; body)`
fn bound_parts(value: Node, bindings: &[(Node, String)], body: Option<Node>) -> Node {
	let subject = symbol(PARTS_WORD);
	let mut statements = match value.drop_meta() {
		Node::Symbol(name) if name == PARTS_WORD => vec![],
		_ => vec![key(subject, Op::Assign, value)],
	};
	statements.extend(bindings.iter().map(|(path, name)| key(Node::Symbol(name.clone()), Op::Assign, path.clone())));
	statements.extend(body);
	Node::List(statements, Bracket::Round, Separator::Semicolon)
}

/// `(a, b)`: the names in order
fn positional_names(target: &Node) -> Option<Vec<String>> {
	let Node::List(items, Bracket::Round, _) = target.drop_meta() else { return None };
	(items.len() > 1).then_some(())?;
	items.iter().map(|item| item.symbol_name().map(String::from)).collect()
}

/// `a, (b = p)` with p an instance
fn last_assigns_instance(items: &[Node], class_of: &ClassOf) -> bool {
	let leading_names = items[..items.len().saturating_sub(1)].iter().all(|item| matches!(item.drop_meta(), Node::Symbol(_)));
	leading_names && items.len() > 1 && matches!(items.last().map(Node::drop_meta), Some(Node::Key(name, Op::Assign, value)) if matches!(name.drop_meta(), Node::Symbol(_)) && class_of(value).is_some())
}


/// A map literal `{x:1 y:2}` or parsed json `parse_json(t)`: what `as Point` builds an instance from
fn is_plain_object(object: &Node) -> bool {
	match object.drop_meta() {
		Node::List(_, Bracket::Curly, _) => true,
		Node::List(items, Bracket::Round, _) => items.first().is_some_and(|word| word.drop_meta().name() == PARSE_JSON_WORD),
		_ => false,
	}
}

/// `int` of `x:int`, `[Point]` of `points:[Point]`, the empty name for an untyped field
fn field_type_name(item: &Node) -> String {
	match item.drop_meta() {
		Node::Key(_, Op::Colon, field_type) => match field_type.drop_meta() {
			Node::Type { name, .. } => name.drop_meta().name(),
			Node::List(items, Bracket::Square, _) if items.len() == 1 => format!("[{}]", items[0].drop_meta().name()),
			// a compound unit, `speed: km/h` (card unit-fields)
			unit @ Node::Key(_, Op::Div | Op::Mul, _) => unit.serialize().trim().to_string(),
			other => other.name(),
		},
		Node::Key(field, Op::Assign, _) => field_type_name(field),
		_ => String::new(),
	}
}

fn with_objects_as_instances(node: Node, fields: &ClassFields) -> Node {
	let class_of = |node: &Node| Some(node.drop_meta().name()).filter(|name| fields.contains_key(name));
	match node {
		// `parse_json(t) as [Point]`: each element an instance
		Node::Key(object, Op::As, list) if matches!(list.drop_meta(), Node::List(items, Bracket::Square, _) if items.len() == 1 && class_of(&items[0]).is_some()) => {
			let class = list.drop_meta().children()[0].drop_meta().name();
			instances_of(with_objects_as_instances(*object, fields), &class, fields)
		}
		// only an object that is no instance yet: `render(t) as docx` picks render's overload (overloads.rs)
		Node::Key(object, Op::As, class) if class_of(&class).is_some() && is_plain_object(&object) => {
			let class = class.drop_meta().name();
			instance_from(with_objects_as_instances(*object, fields), &class, fields)
		}
		// `s as Circle` of a variable or field: the checked downcast (P201); `render(t) as docx` of a call picks an
		// overload (overloads.rs)
		Node::Key(object, Op::As, class) if class_of(&class).is_some() && is_place(&object) => checked_cast(with_objects_as_instances(*object, fields), &class.drop_meta().name()),
		Node::Key(class, Op::Dot, member) if class_of(&class).is_some() && leading_name(&member) == FROM_JSON_WORD => {
			let arguments = match member.drop_meta() {
				Node::List(items, Bracket::Round, _) => items[1..].iter().cloned().map(|argument| with_objects_as_instances(argument, fields)).collect(),
				_ => vec![],
			};
			let parsed = call(PARSE_JSON_WORD, arguments);
			let class = class.drop_meta().name();
			instance_from(parsed, &class, fields)
		}
		other => other.map_children(|child| with_objects_as_instances(child, fields)),
	}
}

/// `s`, `box.shape`, `shapes#1`
fn is_place(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(_) => true,
		Node::Key(holder, Op::Dot | Op::Hash, _) => is_place(holder),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => is_place(&items[0]),
		_ => false,
	}
}

/// `(cast·value = s; if not (cast·value is Circle) { raise "s is no Circle" }; cast·value)`: the value is computed once
fn checked_cast(object: Node, class: &str) -> Node {
	let held = symbol(CAST_VALUE);
	let message = format!("{} is no {class}", object.serialize());
	let check = crate::warp_parser::parse(&format!("if not ({CAST_PLACEHOLDER} is {class}) {{ raise {message:?} }}"));
	let statements = vec![key(held.clone(), Op::Assign, object), crate::library_words::substitute(check, CAST_PLACEHOLDER, &held), held];
	Node::List(statements, Bracket::Round, Separator::Semicolon)
}

fn instance_from(object: Node, class: &str, fields: &ClassFields) -> Node {
	let source = Node::Symbol(format!("{class}{FROM_SUFFIX}"));
	let mut statements = vec![key(source.clone(), Op::Assign, object)];
	statements.extend(statements_of(built_instance(&source, class, fields)));
	Node::List(statements, Bracket::Round, Separator::Semicolon)
}

/// The construction of `instance_of`, after the lists of instances it takes (a construction's arguments are data:
/// the `map` building a list runs before, into `elements·1`, …)
fn built_instance(path: &Node, class: &str, fields: &ClassFields) -> Node {
	let mut lists = vec![];
	let construction = instance_of(path, class, fields, &mut lists);
	match lists.is_empty() {
		true => construction,
		false => Node::List([lists, vec![construction]].concat(), Bracket::Round, Separator::Semicolon),
	}
}

/// `Point(path.x, path.y)`, a field of a class type by its own path, a list of instances by its variable in `lists`
fn instance_of(path: &Node, class: &str, fields: &ClassFields, lists: &mut Vec<Node>) -> Node {
	let mut values = vec![];
	for (field, field_type) in &fields[class] {
		let value = key(path.clone(), Op::Dot, Node::Symbol(field.clone()));
		let element_class = field_type.strip_prefix('[').and_then(|inner| inner.strip_suffix(']')).filter(|inner| fields.contains_key(*inner));
		values.push(match (fields.contains_key(field_type), element_class) {
			(true, _) => instance_of(&value, field_type, fields, lists),
			(false, Some(element_class)) => {
				let elements = Node::Symbol(format!("{ELEMENTS_WORD}·{}", lists.len() + 1));
				lists.push(key(elements.clone(), Op::Assign, instances_of(value, element_class, fields)));
				elements
			}
			(false, None) => value,
		});
	}
	Node::List(std::iter::once(symbol(class)).chain(values).collect(), Bracket::Round, Separator::None)
}

/// `list.map(Point·element => Point(Point·element.x, …))`
fn instances_of(list: Node, class: &str, fields: &ClassFields) -> Node {
	let element = Node::Symbol(format!("{class}{ELEMENT_SUFFIX}"));
	let lambda = key(element.clone(), Op::FatArrow, built_instance(&element, class, fields));
	key(list, Op::Dot, call(MAP_WORD, vec![lambda]))
}

/// A method named like a type word (`double() := x*2`, P142: class methods are always allowed) or like a list
/// mutation (`pop() := items.pop()`) under the name `method·double`, so its calls never read as the conversion
/// `double(c)` or the list's own `xs.pop()`. A type word is renamed wherever it is called as a method; a list word only
/// on what holds an instance of a class defining it (a variable assigned one, `p:Stack`, self) and called bare in a
/// class body, so `s.items.pop()` and `xs.pop()` of lists stay list mutations
fn renamed_type_word_methods(node: Node) -> Node {
	let (mut names, mut classes, mut defined_by) = (vec![], vec![], vec![]);
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		let clashing: Vec<String> = class_items(body).iter().filter_map(method_parts).map(|(name, _, _)| name)
			.filter(|name| !is_witness_method(name) && (crate::analyzer::type_word_kind(name).is_some() || is_library_method(name) || crate::uncertain::INTERVAL_FIELDS.contains(&name.as_str()))).collect();
		if !clashing.is_empty() {
			classes.push(name.drop_meta().name());
		}
		defined_by.extend(clashing.iter().map(|method| (method.clone(), name.drop_meta().name())));
		names.extend(clashing);
	});
	if names.is_empty() {
		return node;
	}
	let mut instances: Vec<String> = instance_classes(&node, &classes).into_keys().collect();
	instances.extend([RECEIVER.to_string(), RECEIVER_ALIASES[0].to_string()]);
	with_method_names(node, &MethodNames { names, instances, classes, defined_by }, false)
}

/// The variables known to hold an instance of one of `classes`, with its class: assigned a construction or a call of a
/// function returning one, annotated `p:Point`, or iterating a list of constructions `for p in ps`
pub(crate) fn instance_classes(node: &Node, classes: &[String]) -> std::collections::HashMap<String, String> {
	let returned = returned_classes(node, classes);
	let constructed_class = |value: &Node| instance_class(value, &returned);
	// `[Point(1, 2), …]`: a list of constructions of one class
	let list_class = |list: &Node| match list.drop_meta() {
		Node::List(items, Bracket::Square, _) if !items.is_empty() => {
			let first = constructed_class(&items[0])?;
			items.iter().all(|item| constructed_class(item).as_ref() == Some(&first)).then_some(first)
		}
		_ => None,
	};
	let mut instances: std::collections::HashMap<String, String> = std::collections::HashMap::new();
	node.visit(&mut |part| if let Node::Key(target, op, value) = part {
		// `p:Point = …`: the annotation says it
		if let (Node::Key(variable, Op::Colon, class), Op::Assign) = (target.drop_meta(), op) {
			// `p:Point`, not a list of them `ps:[Point]`
			let is_class = matches!(class.drop_meta(), Node::Symbol(_) | Node::Type { .. });
			if is_class && classes.contains(&class.drop_meta().name()) {
				instances.insert(variable.drop_meta().name(), class.drop_meta().name());
			}
		}
		let Node::Symbol(variable) = target.drop_meta() else { return };
		let class_of = |operand: &Node| match operand.drop_meta() {
			Node::Symbol(operand) => instances.get(operand).cloned(),
			other => constructed_class(other),
		};
		let class = match (op, value.drop_meta()) {
			// `c = a + b` of an instance a: the operation gives one of a's class, as with_operator_calls reads it
			(Op::Assign | Op::Define, Node::Key(left, Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod, _)) => class_of(left),
			// `w = q as m`, `w = q.to("m")` of a run-time quantity q: another one
			(Op::Assign | Op::Define, conversion) if crate::units::conversion(conversion).is_some() => crate::units::conversion(conversion).and_then(|(quantity, _)| class_of(quantity)),
			(Op::Assign | Op::Define, Node::Key(quantity, Op::Dot, call)) if is_call_of(call, CONVERSION_METHOD) => class_of(quantity),
			// `s = sum([quantity("5 m"), …])`: the sum of instances is one (card quantity-sum)
			(Op::Assign | Op::Define, summed) if summed_list(summed).is_some() => summed_list(summed).and_then(list_class),
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

/// The functions whose value is an instance of one of `classes`, with its class: `make(n) := V(n)`, a body ending in
/// a construction, or in a call of another such function
fn returned_classes(node: &Node, classes: &[String]) -> std::collections::HashMap<String, String> {
	let mut definitions = vec![];
	node.visit(&mut |part| if let Node::Key(head, Op::Define, body) = part {
		if let Node::List(items, Bracket::Round, _) = head.drop_meta() {
			definitions.extend(items.first().map(|name| (name.drop_meta().name(), last_value(body))));
		}
	});
	let mut returned = std::collections::HashMap::new();
	// a function returning another's result: until no function is added
	loop {
		let known = returned.len();
		for (function, value) in &definitions {
			let class = constructed_class(value).and_then(|name| if classes.contains(&name) { Some(name) } else { returned.get(&name).cloned() });
			if let Some(class) = class {
				returned.entry(function.clone()).or_insert(class);
			}
		}
		if returned.len() == known {
			return returned;
		}
	}
}

/// The class of an instance a construction or a call of a function of `returned` gives
fn instance_class(value: &Node, returned: &std::collections::HashMap<String, String>) -> Option<String> {
	constructed_class(value).map(|name| returned.get(&name).cloned().unwrap_or(name))
}

/// The value of a body: its last statement
fn last_value(body: &Node) -> &Node {
	match body.drop_meta() {
		Node::List(items, Bracket::Curly, _) if !items.is_empty() => last_value(&items[items.len() - 1]),
		other => other,
	}
}

/// The class a construction `Point(1, 2)`, `Point{…}` builds (by its name, before the constructions are lowered)
fn constructed_class(value: &Node) -> Option<String> {
	match value.drop_meta() {
		Node::List(items, Bracket::Round, _) => items.first().map(|first| first.drop_meta().name()),
		Node::Key(class, Op::None | Op::Colon, _) => Some(class.drop_meta().name()),
		_ => None,
	}
}
/// The clashing method names, and the variables holding instances of the classes defining them
struct MethodNames {
	names: Vec<String>,
	instances: Vec<String>,
	classes: Vec<String>,
	/// (method, class) of every renamed method
	defined_by: Vec<(String, String)>,
}

/// The receiver of no known class calls a library-word method (`x.count()`) that classes define: the class's method
/// when x is one of them at run time, else the library word (card class-method)
fn dispatched_by_class(receiver: &Node, member: Node, methods: &MethodNames, renamed_member: Node) -> Node {
	let name = leading_name(&member);
	let library_call = key(receiver.clone(), Op::Dot, member);
	let method_call = key(receiver.clone(), Op::Dot, renamed_member);
	methods.defined_by.iter().filter(|(method, _)| *method == name).fold(library_call, |otherwise, (_, class)| {
		let bindings = [("RECEIVER", receiver.clone()), ("CLASS", Node::Symbol(class.clone())), ("METHOD", method_call.clone()), ("OTHERWISE", otherwise)];
		let bindings = bindings.into_iter().map(|(placeholder, node)| (placeholder.to_string(), node)).collect();
		crate::law::substitute(&crate::warp_parser::parse(DISPATCH_TEMPLATE), &bindings).drop_meta().clone()
	})
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
		match is_library_method(&name) || crate::uncertain::INTERVAL_FIELDS.contains(&name.as_str()) {
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
			key(recurse(*receiver), Op::Dot, member)
		}
		// `x.count()` of an x of no known class, outside the classes (whose fields have declared types); a list mutation
		// (`xs.pop()`) stays the list's
		Node::Key(receiver, Op::Dot, member) if !in_class && matches!(receiver.drop_meta(), Node::Symbol(variable) if !methods.instances.contains(variable)) && !crate::analyzer::is_list_mutating_method(&leading_name(&member)) && (renamed(&member).is_some() || renamed_call(&member).is_some()) => {
			let member = with_arguments(*member, recurse);
			let renamed_member = renamed(&member).or_else(|| renamed_call(&member)).expect("guarded");
			dispatched_by_class(&receiver, member, methods, renamed_member)
		}
		// `items.pop()` of a list: the member keeps its name, its arguments may call methods
		Node::Key(receiver, Op::Dot, member) => key(recurse(*receiver), Op::Dot, with_arguments(*member, recurse)),
		Node::List(..) if in_class && renamed_call(&node).is_some() => {
			let Node::List(items, bracket, separator) = renamed_call(&node).expect("guarded") else { unreachable!("a call") };
			Node::List(items.into_iter().map(recurse).collect(), bracket, separator)
		}
		// `pop() := …`, or a getter `double := …`
		Node::Key(head, Op::Define, body) if in_class && (renamed(&head).is_some() || renamed_call(&head).is_some()) => {
			let head = renamed(&head).or_else(|| renamed_call(&head)).expect("guarded");
			key(head, Op::Define, recurse(*body))
		}
		other => other.map_children(recurse),
	}
}

/// The call `name(args)` with each argument passed through `change`, the name kept
fn with_arguments(member: Node, change: impl Fn(Node) -> Node) -> Node {
	match member {
		Node::List(items, Bracket::Round, separator) if !items.is_empty() => {
			Node::List(items[..1].iter().cloned().chain(items[1..].iter().cloned().map(change)).collect(), Bracket::Round, separator)
		}
		other => other,
	}
}

/// Kotlin's `p.copy(y = 5)`: a new instance with p's fields, those changed: `field_with(instance_copy(p, no), "y", 5)`;
/// `shallow = yes` among them asks for the shallow copy (P205)
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
				Some(mut changes) => {
					let shallow = changes.iter().position(|(field, _)| field == crate::library_words::SHALLOW).map_or(Node::False, |at| changes.remove(at).1);
					changes.into_iter().fold(crate::library_words::copy_call(receiver, shallow), |object, (field, value)| {
						let call = vec![symbol(crate::library_words::FIELD_WITH), object, Node::Text(field), value];
						Node::List(call, Bracket::Round, Separator::None)
					})
				}
				None => key(receiver, Op::Dot, member),
			}
		}
		other => other.map_children(copies),
	}
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
	items.into_iter().map(without_modifiers).map(with_body_words).flat_map(|item| match item.drop_meta() {
		Node::List(words, _, _) if keyword_method(words).is_some() => keyword_method(words).into_iter().collect(),
		Node::List(words, _, _) if braced_method(words).is_some() => braced_method(words).into_iter().collect(),
		Node::List(words, _, _) if value_block(words).is_some() => value_block(words).into_iter().collect(),
		Node::List(words, _, _) if accessors(words).is_some() => accessors(words).unwrap_or_default(),
		Node::List(..) if included_mixin(&item).is_some() => vec![item],
		Node::List(words, _, _) if nested_fields(words).is_some() => nested_fields(words).into_iter().collect(),
		// Java's `int x`: the field x of type int; C#'s auto-property `int X { get; set; }` the field X
		Node::List(words, _, _) if typed_field(words).is_some() => typed_field(words).into_iter().collect(),
		Node::List(words, _, _) if auto_property(words).is_some() => auto_property(words).into_iter().collect(),
		// TypeScript's `twice(): number { … }`
		Node::Key(call, Op::Colon, typed_block) if typed_method(call, typed_block).is_some() => typed_method(call, typed_block).into_iter().collect(),
		Node::List(group, Bracket::None, _) => group.clone(),
		_ => vec![item],
	}).map(as_member).collect()
}

/// `fn f() := 3 squared`: the parser ends the definition before a suffix word, `(f() := 3) squared`; the words after it
/// are its body's, as ambiguous_forms regroups them outside a class
fn with_body_words(item: Node) -> Node {
	let Node::List(words, Bracket::None, Separator::Space) = item.drop_meta() else { return item };
	let Some(at) = words.iter().position(|word| matches!(word.drop_meta(), Node::Key(head, Op::Define, _) if matches!(head.drop_meta(), Node::List(_, Bracket::Round, _)))) else { return item };
	let (before, rest) = words.split_at(at + 1);
	if rest.is_empty() {
		return item;
	}
	let Node::Key(head, op, value) = before[at].drop_meta() else { return item };
	let body = Node::List([vec![value.as_ref().clone()], rest.to_vec()].concat(), Bracket::None, Separator::Space);
	let definition = Node::Key(head.clone(), *op, Box::new(body));
	Node::List([before[..at].to_vec(), vec![definition]].concat(), Bracket::None, Separator::Space)
}

/// `def area() -> int {…}`, `fun area(): Int {…}`, `func area() {…}`: the method `area() := …` (with its result type)
fn keyword_method(words: &[Node]) -> Option<Node> {
	// Kotlin's expression body `fun sum() = x + y` defines as `:=` does; Java's `int sum() {…}` as C's
	match crate::declarations::keyword_definition(words).or_else(|| crate::declarations::c_function(words)).or_else(|| python_method(words))? {
		Node::Key(head, Op::Assign, body) => Some(Node::Key(head, Op::Define, body)),
		definition => Some(definition),
	}
}

/// Python's `def f(): x + 1` without parameters, which keyword_definition leaves to late_binding outside a class (P71):
/// in a class body it is the method `f() := x + 1`
fn python_method(words: &[Node]) -> Option<Node> {
	let [keyword, definition] = words else { return None };
	let is_keyword = matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word));
	match definition.drop_meta() {
		Node::Key(head, Op::Colon, body) if is_keyword && matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_)))) => {
			Some(Node::Key(head.clone(), Op::Define, body.clone()))
		}
		_ => None,
	}
}

/// A property's getter and setter as methods: `get age() {…}` is the getter `age := …`, `set age(v) {…}` the setter
/// `age·set(v) := …`; the warp form `age:{getter} set{setter}` has the new value as `it`
fn accessors(words: &[Node]) -> Option<Vec<Node>> {
	let [first, second] = words else { return None };
	let method = |name: &str, parameters: Vec<Node>, body: &Node| {
		let head = match parameters.is_empty() {
			true => symbol(name),
			false => call(name, parameters),
		};
		key(head, Op::Define, body.clone())
	};
	let is_block = |node: &Node| matches!(node.drop_meta(), Node::List(_, Bracket::Curly, _));
	match (first.drop_meta(), second.drop_meta()) {
		// `get age() {…}`, `set age(v) {…}`
		(Node::Symbol(word), Node::List(parts, _, _)) if crate::warp_parser::ACCESSOR_WORDS.contains(&word.as_str()) => {
			let [call, body] = parts.as_slice() else { return None };
			let Node::List(call, Bracket::Round, _) = call.drop_meta() else { return None };
			let name = call.first()?.drop_meta().name();
			let parameters: Vec<Node> = call[1..].iter().flat_map(grouped_parameters).collect();
			match word == "get" {
				true => Some(vec![method(&name, vec![], body)]),
				false => Some(vec![method(&format!("{name}{SETTER_SUFFIX}"), parameters, body)]),
			}
		}
		// `age:{2026 - birthday} set{birthday = 2026 - it}`
		(Node::Key(name, Op::Colon | Op::None, getter), Node::Key(set, Op::Colon | Op::None, setter)) if set.drop_meta().name() == "set" && is_block(getter) && is_block(setter) => {
			let name = name.drop_meta().name();
			// `it` is the new value; named apart, so no pass reads the block as a lambda of `it`
			let new_value = Node::Symbol(format!("{}{SETTER_SUFFIX}", crate::warp_parser::IT_WORD));
			let setter = renamed_symbol(setter.as_ref().clone(), crate::warp_parser::IT_WORD, &new_value);
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
			_ => key(recurse(*target), Op::Assign, recurse(*value)),
		},
		other => other.map_children(recurse),
	}
}

/// A member without the words that change nothing in warp: `mutating func f() {…}` is `func f() {…}`, Swift's
/// `var count = 0` the field `count = 0`
fn without_modifiers(item: Node) -> Node {
	let Node::List(words, bracket, separator) = item.drop_meta().clone() else { return item };
	let is_modifier = |word: &Node| matches!(word.drop_meta(), Node::Symbol(word) if crate::warp_parser::MEMBER_MODIFIERS.contains(&word.as_str()) || crate::warp_parser::FIELD_KEYWORDS.contains(&word.as_str()));
	let kept: Vec<Node> = words.iter().skip_while(|word| is_modifier(word)).cloned().collect();
	if let Some(kept_word) = kept.first().map(leading_name).filter(|_| kept.len() < words.len()) {
		let modifiers: Vec<String> = words[..words.len() - kept.len()].iter().map(|word| word.drop_meta().name()).collect();
		if !modifiers.iter().any(|word| SILENT_MODIFIERS.contains(&word.as_str())) {
			crate::normalize::set_position_of(&item);
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

/// wiki/class.md's `address { street; city; zip? }`: the field address holding a block of fields, `address:{…}`
fn nested_fields(words: &[Node]) -> Option<Node> {
	let [name, block] = words else { return None };
	let Node::Symbol(word) = name.drop_meta() else { return None };
	let Node::List(fields, Bracket::Curly, _) = block.drop_meta() else { return None };
	let is_field = |field: &Node| match field.drop_meta() {
		Node::Symbol(_) => true,
		Node::Key(_, Op::Colon, _) => true,
		Node::List(group, Bracket::None, _) => group.iter().all(|field| matches!(field.drop_meta(), Node::Symbol(_) | Node::Key(_, Op::Colon, _))),
		_ => false,
	};
	let is_member_word = crate::operators::is_function_keyword(word) || is_constructor_word(word) || crate::warp_parser::ACCESSOR_WORDS.contains(&word.as_str());
	(!is_member_word && !fields.is_empty() && fields.iter().all(is_field)).then(|| key(name.clone(), Op::Colon, block.clone()))
}

/// Java's and C#'s field `int x`, Go's `x int`: the field `x:int`
fn typed_field(words: &[Node]) -> Option<Node> {
	let is_type = |word: &Node| crate::analyzer::type_word_kind(&word.drop_meta().name()).is_some();
	let field = |name: &Node, field_type: &Node| key(name.clone(), Op::Colon, field_type.clone());
	match words {
		[field_type, name] if is_type(field_type) && matches!(name.drop_meta(), Node::Symbol(_)) => Some(field(name, field_type)),
		[name, field_type] if is_type(field_type) && matches!(name.drop_meta(), Node::Symbol(_)) => Some(field(name, field_type)),
		_ => None,
	}
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
	(is_call && is_block).then(|| key(key(call.clone(), Op::Colon, result_type.clone()), Op::Define, block.clone()))
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
				true => item.with_attribute(crate::warp_parser::STATIC_KEYWORD, Node::True),
				false => item,
			}).collect();
			Node::Type { name, body: Box::new(Node::List(items, Bracket::Curly, Separator::Semicolon)) }
		}
		other => other.map_children(|child| class_attributes(child, qualified)),
	}
}

/// The names of the classes the program declares whose body `keep` accepts
fn declared_classes(node: &Node, keep: impl Fn(&Node) -> bool) -> Vec<String> {
	let mut classes = vec![];
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		if keep(body) {
			classes.push(name.drop_meta().name());
		}
	});
	classes
}

/// Go's positional braces `Point{1, 2}` of a declared class: the construction `Point(1, 2)`, with a note (P167); of
/// an unknown name they stay tagged data
fn positional_braces(node: Node) -> Node {
	let classes = declared_classes(&node, |_| true);
	if classes.is_empty() {
		return node;
	}
	constructed_from_braces(node, &classes)
}

fn constructed_from_braces(node: Node, classes: &[String]) -> Node {
	let is_positional = |items: &[Node]| !items.is_empty() && items.iter().all(|item| !matches!(item.drop_meta(), Node::Key(..)));
	match node {
		Node::Key(class, Op::None, braces) if matches!(class.drop_meta(), Node::Symbol(name) if classes.contains(name)) && matches!(braces.drop_meta(), Node::List(items, Bracket::Curly, _) if is_positional(items)) => {
			let Node::List(items, _, _) = braces.drop_meta().clone() else { unreachable!("guarded") };
			let class_name = class.drop_meta().name();
			let written: Vec<String> = items.iter().map(Node::serialize).collect();
			crate::normalize::set_position_of(&class);
			crate::diagnostic::note_alias(&format!("{class_name}{{{}}}", written.join(", ")), &format!("{class_name}({})", written.join(", ")));
			let arguments = items.into_iter().map(|item| constructed_from_braces(item, classes));
			Node::List([vec![*class]].into_iter().flatten().chain(arguments).collect(), Bracket::Round, Separator::None)
		}
		// a match arm's `Point{x, y} =>` is a pattern (destructurings), no construction
		Node::Key(pattern, Op::FatArrow, body) => Node::Key(pattern, Op::FatArrow, Box::new(constructed_from_braces(*body, classes))),
		other => other.map_children(|child| constructed_from_braces(child, classes)),
	}
}

/// Every class's constructor as `init(…){…}` (P162): one named by another language (`constructor(x)`, `__init__`,
/// CONSTRUCTOR_ALIASES) or like its class (Java's `Point(int x, int y) {…}`), with a got-it note; an alias the program
/// also calls as a method (`p.new(2)`) stays a method
fn with_init_constructors(node: Node, called: &std::collections::HashSet<String>) -> Node {
	match node {
		Node::Type { name, body } => {
			let class = name.drop_meta().name();
			let is_alias = |item: &Node| constructor_alias(item).filter(|alias| *alias == class || ((CONSTRUCTOR_ALIASES.contains(&alias.as_str()) || *alias == CONSTRUCTOR_WORD) && !called.contains(alias)));
			let items: Vec<Node> = class_items(&body);
			if !items.iter().any(|item| is_alias(item).is_some()) {
				return Node::Type { name, body };
			}
			let items = items.into_iter().map(|item| match is_alias(&item) {
				// `init(xs) := {…}` is the constructor too, without a note
				Some(alias) if alias == CONSTRUCTOR_WORD => as_init(item),
				Some(alias) => {
					crate::normalize::set_position_of(&item);
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
	let word = call(CONSTRUCTOR_WORD, parameters);
	key(word, Op::None, body)
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
		Node::Type { name, body } if name.attribute(crate::warp_parser::MIXIN_WORD).is_none() => {
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
	let classes = declared_classes(&node, |_| true);
	if classes.is_empty() {
		return node;
	}
	declared_with_classes(node, &classes)
}

fn declared_with_classes(node: Node, classes: &[String]) -> Node {
	match node {
		// not the call `P(x = 7)`, a construction with a named argument
		Node::List(words, bracket, _) if bracket != Bracket::Round && words.len() == 2 && classes.contains(&words[0].drop_meta().name()) && matches!(words[1].drop_meta(), Node::Key(variable, Op::Assign, _) if matches!(variable.drop_meta(), Node::Symbol(_))) => {
			let Node::Key(variable, _, value) = words[1].drop_meta().clone() else { unreachable!("guarded") };
			let typed = Node::Key(variable, Op::Colon, Box::new(words[0].clone()));
			key(typed, Op::Assign, declared_with_classes(*value, classes))
		}
		other => other.map_children(|child| declared_with_classes(child, classes)),
	}
}

/// JavaScript's `sum() { return … }`: the method `sum() := {…}`
fn braced_method(words: &[Node]) -> Option<Node> {
	let [call, block] = words else { return None };
	let is_call = matches!(call.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if !is_constructor_word(name)));
	let is_block = matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _));
	(is_call && is_block).then(|| key(call.clone(), Op::Define, block.clone()))
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
	let parameters: Vec<Node> = parameters.iter().flat_map(grouped_parameters).collect();
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
	let name = glyph_method(&name.drop_meta().name()).map_or(name.clone(), symbol);
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
	(is_block && constructor_parameters(word).is_some()).then(|| key(word.clone(), Op::None, block.clone()))
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
	collect_fields_set(body, parameters, &mut names);
	names
}

/// The fields `fields_set` finds; what a loop sets (`for x in xs { i = … }`) stays local to the constructor
fn collect_fields_set(node: &Node, parameters: &[String], names: &mut Vec<String>) {
	if crate::event_signals::is_loop(node) {
		return;
	}
	match node.drop_meta() {
		Node::Key(target, op, value) => {
			let name = match target.drop_meta() {
				_ if *op != Op::Assign => None,
				Node::Symbol(name) if !parameters.contains(name) => Some(name.clone()),
				Node::Key(receiver, Op::Dot, field) if RECEIVER_ALIASES.contains(&receiver.drop_meta().name().as_str()) || receiver.drop_meta().name() == RECEIVER => Some(field.drop_meta().name()),
				_ => None,
			};
			if let Some(name) = name.filter(|name| !names.contains(name)) {
				names.push(name);
			}
			collect_fields_set(target, parameters, names);
			collect_fields_set(value, parameters, names);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| collect_fields_set(item, parameters, names)),
		_ => {}
	}
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

/// `trait has·area{area}` for each method name that two or more classes define, taking the first one's parameters
fn shared_method_traits(node: &Node) -> Vec<Node> {
	let mut methods: Vec<(String, Node)> = vec![];
	node.visit(&mut |part| {
		let Node::Type { body, .. } = part else { return };
		for item in class_items(body) {
			if let Some((name, parameters)) = instance_member(&item) {
				let receiver = symbol(RECEIVER);
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
			let declaration = key(Node::Symbol(format!("{IMPLICIT_TRAIT_PREFIX}{name}")), Op::Colon, requirements);
			traits.push(Node::List(vec![symbol(TRAIT_KEYWORD), declaration], Bracket::None, Separator::Space));
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
	item.attribute(crate::warp_parser::STATIC_KEYWORD).is_some()
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
	let Node::Symbol(class) = object.drop_meta() else { return key(recurse(*object), Op::Dot, recurse(*member)) };
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
/// the update `x = m(x, args)`; each call's pair is a temp of its own, `pop·result·1`, so a call inside a method shares
/// no name with one in main
fn method_calls(node: Node, called: &[String], changing: &Changing, calls: &Counter) -> Node {
	let recurse = |child: Node| method_calls(child, called, changing, calls);
	let Node::Key(receiver, Op::Dot, member) = node else { return node.map_children(recurse) };
	let receiver = recurse(*receiver);
	let (name, arguments) = match member.drop_meta() {
		Node::Symbol(name) => (name.clone(), vec![]),
		Node::List(items, Bracket::Round, _) if !items.is_empty() => (items[0].drop_meta().name(), items[1..].iter().cloned().map(recurse).collect()),
		_ => return key(receiver, Op::Dot, recurse(*member)),
	};
	if !called.contains(&name) && !changing.contains(&name) {
		return key(receiver, Op::Dot, recurse(*member));
	}
	let call = Node::List([Node::Symbol(name.clone()), receiver.clone()].into_iter().chain(arguments).collect(), Bracket::Round, Separator::None);
	let item = |pair: Node, position: i64| key(pair, Op::Hash, Node::int(position));
	match receiver.drop_meta() {
		// `s.pop()`: the pair (value, changed object) of the call, the object stored back, the value given
		Node::Symbol(_) if changing.giving_value.contains(&name) => {
			let pair = Node::Symbol(format!("{name}{RESULT_SUFFIX}·{}", calls.next_number()));
			Node::List(vec![
				key(pair.clone(), Op::Assign, call),
				key(receiver, Op::Assign, item(pair.clone(), 2)),
				item(pair, 1),
			], Bracket::Round, Separator::Semicolon)
		}
		_ if changing.giving_value.contains(&name) => item(call, 1),
		Node::Symbol(_) if changing.itself.contains(&name) => key(receiver, Op::Assign, call),
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
	let gives_itself = |body: &Node| Node::List(vec![body.clone(), symbol(RECEIVER)], Bracket::None, Separator::Semicolon);
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
		let head = call(&static_name(class, &name), parameters);
		return vec![key(head, Op::Define, receiver_reads(body, &readable))];
	}
	let Some(name) = field_name(member) else { return vec![] };
	let Node::Key(_, Op::Assign, value) = member.drop_meta() else { return vec![] };
	let global = Node::Symbol(static_name(class, &name));
	let receiver = key(symbol(RECEIVER), Op::Colon, symbol(class));
	let getter_head = call(&name, vec![receiver]);
	let declaration = Node::Key(Box::new(global.clone()), Op::Assign, value.clone());
	// `global c·n = 0`: methods may change it (`n += 1`)
	let declaration = key(symbol(GLOBAL_WORD), Op::Colon, declaration);
	vec![declaration, key(getter_head, Op::Define, global)]
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
	// the holes of `"#\(n)"` as code first, so a field in one is read from self too
	let body = crate::interpolation::lower_program(body);
	let body = receiver_reads(body, &readable);
	let receiver = symbol(RECEIVER);
	let changes = match changes_fields_of(&body, RECEIVER) {
		false => Change::None,
		true if gives_value(&body) => Change::GivingValue,
		true => Change::Itself,
	};
	let body = match changes {
		Change::None => body,
		Change::Itself => Node::List(vec![with_returns(body, &|_| receiver.clone()), receiver], Bracket::None, Separator::Semicolon),
		Change::GivingValue => {
			// an early `return v` gives the pair too: `if n <= 0 { return 0 }; n -= 1; n`
			let paired = |value: Node| Node::List(vec![value, receiver.clone()], Bracket::Square, Separator::Space);
			// the statements before the value run first: `x = items#1; items = …; x` gives x, not the block
			let mut statements = statements_of(with_returns(body, &paired));
			let value = statements.pop().unwrap_or(Node::Empty);
			let result = Node::Symbol(format!("{method}{VALUE_SUFFIX}"));
			let pair = Node::List(vec![result.clone(), receiver], Bracket::Square, Separator::Space);
			statements.extend([key(result, Op::Assign, value), pair]);
			Node::List(statements, Bracket::None, Separator::Semicolon)
		}
	};
	let receiver = key(symbol(RECEIVER), Op::Colon, symbol(class));
	let head = Node::List([vec![symbol(method), receiver], parameters].concat(), Bracket::Round, Separator::None);
	(key(head, Op::Define, body), changes)
}

/// Each `return v` of the body (not of a function or lambda inside it) as `return given(v)`
fn with_returns(node: Node, given: &dyn Fn(Node) -> Node) -> Node {
	let is_return = |word: &Node| word.is_symbol(RETURN_WORD);
	match node {
		Node::Symbol(_) if is_return(&node) => Node::List(vec![node, given(Node::Empty)], Bracket::None, Separator::Space),
		Node::List(items, bracket, separator) if items.len() <= 2 && items.first().is_some_and(is_return) => {
			let value = items.get(1).cloned().unwrap_or(Node::Empty);
			Node::List(vec![items[0].clone(), given(value)], bracket, separator)
		}
		Node::Key(_, Op::Define | Op::FatArrow | Op::Arrow, _) => node,
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
	let Some(last) = statements_of(body.clone()).pop() else { return false };
	match last.drop_meta() {
		// `if c {items.add(x)}`: a branch gives what its last statement gives
		Node::Key(_, Op::Then, branch) => gives_value(branch),
		Node::Key(then, Op::Else, otherwise) => gives_value(then) || gives_value(otherwise),
		// a nested sequence gives what its last statement gives
		Node::List(_, Bracket::Round, Separator::Semicolon) | Node::List(_, Bracket::Curly, _) => gives_value(&last),
		Node::Key(_, op, _) if matches!(op, Op::Assign | Op::Define | Op::Inc | Op::Dec) || op.is_compound_assign() => false,
		// a body giving its object back already (a constructor)
		Node::Symbol(name) if name == RECEIVER => false,
		Node::Key(_, Op::Dot, call) => !mutating_call(call) || matches!(call.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|word| GIVING_MUTATIONS.contains(&word.drop_meta().name().as_str()))),
		_ => true,
	}
}

/// The statements of a block `{a; b}` or sequence `a; b`, or the one statement of any other body
fn statements_of(body: Node) -> Vec<Node> {
	match body.drop_meta() {
		Node::List(statements, Bracket::Curly, _) | Node::List(statements, Bracket::None | Bracket::Round, Separator::Semicolon | Separator::Newline) => statements.clone(),
		_ => vec![body],
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
	let on_receiver = |member: Node| key(symbol(RECEIVER), Op::Dot, member);
	let is_call_of = |items: &[Node], names: &[&String]| matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if names.contains(&name));
	match node {
		Node::Symbol(name) if readable.statics.contains(&&name) => Node::Symbol(static_name(readable.class, &name)),
		Node::List(items, Bracket::Round, separator) if is_call_of(&items, &readable.statics) => {
			let arguments = items[1..].iter().cloned().map(|argument| receiver_reads(argument, readable));
			let function = Node::Symbol(static_name(readable.class, &items[0].drop_meta().name()));
			Node::List(std::iter::once(function).chain(arguments).collect(), Bracket::Round, separator)
		}
		Node::Symbol(name) if readable.fields.contains(&&name) => on_receiver(Node::Symbol(name)),
		Node::Symbol(name) if RECEIVER_ALIASES.contains(&name.as_str()) => symbol(RECEIVER),
		Node::List(items, Bracket::Round, separator) if is_call_of(&items, &readable.methods) => {
			let arguments = items[1..].iter().cloned().map(|argument| receiver_reads(argument, readable));
			on_receiver(Node::List(std::iter::once(items[0].clone()).chain(arguments).collect(), Bracket::Round, separator))
		}
		Node::Key(object, Op::Dot, member) => Node::Key(Box::new(receiver_reads(*object, readable)), Op::Dot, member),
		other => other.map_children(|child| receiver_reads(child, readable)),
	}
}

/// Does the body assign a field of the variable (`self.n = …`, `self.n += 1`, `self.n++`) or change a list in one
/// (`self.items.add(x)`)
fn changes_fields_of(body: &Node, variable: &str) -> bool {
	let mut changes = false;
	body.visit(&mut |part| {
		changes |= match part {
			Node::Key(target, Op::Dot, call) if is_field_of(target, variable) => mutating_call(call),
			Node::Key(target, op, _) => (*op == Op::Assign || op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec)) && is_field_of(target, variable),
			_ => false,
		};
	});
	changes
}

/// `self.items`, `self.stack.items`, an element `self.counts#i`
fn is_field_of(target: &Node, variable: &str) -> bool {
	match target.drop_meta() {
		Node::Key(object, Op::Dot, _) => matches!(object.drop_meta(), Node::Symbol(name) if name == variable) || is_field_of(object, variable),
		Node::Key(list, Op::Hash, _) => is_field_of(list, variable),
		_ => false,
	}
}

/// `add(x)`, `insert(x, at:1)`: a call of a method that changes the list it is called on
fn mutating_call(call: &Node) -> bool {
	matches!(call.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if crate::analyzer::is_list_mutating_method(name)))
}
