//! Methods in a class body (issue #14, notes/classes.md): `class person{name; greet() := "hi " + name}` is the class
//! `person{name}` and the function `greet(self:person) := "hi " + self.name`. In a method body a bare field name and
//! `self`/`this` read the receiver; `p.greet()`, `greet(p)` and, for a method without parameters, `p.area` call it.
//! A method name that several classes define is declared as an implicit trait, so a call picks the class's own method
//! by the static type of its receiver (traits.rs witnesses).
//! `class dog extends animal {…}` (P117) gives dog the fields and methods of animal, its own ones override them, and
//! declares `dog like animal`, so a dog is accepted where an animal is wanted.

use super::nodes::{call, key};
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
const GLOBAL_KEYWORD: &str = "global";
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
const RETURN_WORD: &str = "return";
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
/// `len(s)`, `count(s)`, `s.count()` of an instance whose class defines its size under another of these names
const SIZE_WORDS: [&str; 4] = ["size", "count", "len", "length"];
/// The keywords of a field: Swift's `var count = 0`, `let`, Kotlin's `val`
const FIELD_KEYWORDS: [&str; 3] = ["var", "let", "val"];
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
const SUM_WORD: &str = "sum";
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
	let node = if shared.is_empty() && changing.is_empty() { node } else { method_calls(node, &shared, &changing, &std::cell::Cell::new(0)) };
	match (traits.is_empty(), node) {
		(true, node) => node,
		(false, Node::List(items, bracket, separator)) if separator != Separator::Space => Node::List([traits, items].concat(), bracket, separator),
		(false, node) => Node::List([traits, vec![node]].concat(), Bracket::None, Separator::Semicolon),
	}
}

/// `d.append(1)` of a deque, `s.contains(2)` of a set, `len(s)`: the class's own method when it does not define that
/// name (METHOD_ALIASES, SIZE_WORDS), with a note
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
	let candidates: Vec<&str> = match SIZE_WORDS.contains(&written) {
		true => SIZE_WORDS.to_vec(),
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
		// `len(s)`: `s.size()`
		Node::List(items, Bracket::Round, separator) if matches!(items.as_slice(), [word, argument] if SIZE_WORDS.contains(&word.drop_meta().name().as_str()) && defined_by(argument).is_some()) => {
			match aliased_method(&items[0].drop_meta().name(), defined_by(&items[1]).expect("guarded")) {
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
const MAP_WORD: &str = "map";
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
pub(crate) fn class_fields(node: &Node) -> std::collections::HashMap<String, Vec<(String, String)>> {
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
	let fields = class_fields(&node);
	let classes: Vec<String> = fields.keys().cloned().collect();
	let instances = instance_classes(&node, &classes);
	let class_of = |value: &Node| match value.drop_meta() {
		Node::Symbol(variable) => instances.get(variable).cloned(),
		other => constructed_class(other).filter(|class| fields.contains_key(class)),
	};
	taken_apart(node, &fields, &class_of)
}

/// P179: a field by its position, `c#1` 1-based and Rust's `c.0` 0-based, of an instance whose class is known
/// (`c = rgb(1, 2, 3)`, `Some(4)#1`): the field by name, `c.value1`
fn positional_fields(node: Node) -> Node {
	let fields = class_fields(&node);
	let classes: Vec<String> = fields.keys().cloned().collect();
	let instances = instance_classes(&node, &classes);
	let class_of = |value: &Node| match value.drop_meta() {
		Node::Symbol(variable) => instances.get(variable).cloned(),
		other => constructed_class(other).filter(|class| fields.contains_key(class)),
	};
	by_position(node, &fields, &class_of)
}

fn by_position(node: Node, fields: &std::collections::HashMap<String, Vec<(String, String)>>, class_of: &dyn Fn(&Node) -> Option<String>) -> Node {
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

fn taken_apart(node: Node, fields: &std::collections::HashMap<String, Vec<(String, String)>>, class_of: &dyn Fn(&Node) -> Option<String>) -> Node {
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
			let guard = key(key(Node::Empty, Op::If, test), Op::Then, subject.clone());
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
fn is_class_pattern(pattern: &Node, fields: &std::collections::HashMap<String, Vec<(String, String)>>) -> bool {
	match pattern.drop_meta() {
		Node::Key(class, Op::Colon | Op::None, parts) => fields.contains_key(&class.drop_meta().name()) && matches!(parts.drop_meta(), Node::List(_, Bracket::Curly, _)),
		Node::List(items, Bracket::Round, _) => items.len() > 1 && fields.contains_key(&items[0].drop_meta().name()),
		_ => false,
	}
}

/// A pattern at `path`: `_` matches anything, a name binds, a number or text is compared, `Point{x, y: b}` and
/// `Point(a, 0)` test the class and match each field; None for anything else (the arm stays as written)
fn matched_pattern(pattern: &Node, path: &Node, fields: &std::collections::HashMap<String, Vec<(String, String)>>, matched: &mut Matched) -> Option<()> {
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
fn matched_fields(parts: &Node, path: &Node, fields: &std::collections::HashMap<String, Vec<(String, String)>>) -> Option<Matched> {
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
	items.iter().map(|item| match item.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		_ => None,
	}).collect()
}

/// `a, (b = p)` with p an instance
fn last_assigns_instance(items: &[Node], class_of: &dyn Fn(&Node) -> Option<String>) -> bool {
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

fn with_objects_as_instances(node: Node, fields: &std::collections::HashMap<String, Vec<(String, String)>>) -> Node {
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

fn instance_from(object: Node, class: &str, fields: &std::collections::HashMap<String, Vec<(String, String)>>) -> Node {
	let source = Node::Symbol(format!("{class}{FROM_SUFFIX}"));
	let mut statements = vec![key(source.clone(), Op::Assign, object)];
	statements.extend(statements_of(built_instance(&source, class, fields)));
	Node::List(statements, Bracket::Round, Separator::Semicolon)
}

/// The construction of `instance_of`, after the lists of instances it takes (a construction's arguments are data:
/// the `map` building a list runs before, into `elements·1`, …)
fn built_instance(path: &Node, class: &str, fields: &std::collections::HashMap<String, Vec<(String, String)>>) -> Node {
	let mut lists = vec![];
	let construction = instance_of(path, class, fields, &mut lists);
	match lists.is_empty() {
		true => construction,
		false => Node::List([lists, vec![construction]].concat(), Bracket::Round, Separator::Semicolon),
	}
}

/// `Point(path.x, path.y)`, a field of a class type by its own path, a list of instances by its variable in `lists`
fn instance_of(path: &Node, class: &str, fields: &std::collections::HashMap<String, Vec<(String, String)>>, lists: &mut Vec<Node>) -> Node {
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
fn instances_of(list: Node, class: &str, fields: &std::collections::HashMap<String, Vec<(String, String)>>) -> Node {
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

/// `a + b` of an instance a whose class defines the operator's method (`plus`, Python's `__add__`, wiki/operator.md):
/// the method call `a.plus(b)`; an operation of such calls is an instance of the same class (`a + b + c`)
fn operator_calls(node: Node) -> Node {
	let mut methods: Vec<(String, Op, String)> = vec![];
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		for (method, _, _) in class_items(body).iter().filter_map(method_parts) {
			if let Some((op, names)) = OPERATOR_METHODS.iter().find(|(_, names)| names.contains(&method.as_str())) {
				if method != names[0] {
					crate::normalize::set_position_of(part);
					crate::diagnostic::note_alias(&method, names[0]);
				}
				methods.push((name.drop_meta().name(), *op, method));
			}
		}
	});
	if methods.is_empty() {
		return node;
	}
	let defines = |methods: &[(String, Op, String)], class: &str, op: Op| methods.iter().find(|(owner, known, _)| owner == class && *known == op).map(|(_, _, method)| method.clone());
	for (class, _, _) in methods.clone() {
		for (defined, missing) in INTERCHANGEABLE_OPERATORS {
			if let (Some(method), None) = (defines(&methods, &class, defined), defines(&methods, &class, missing)) {
				methods.push((class.clone(), missing, method));
			}
		}
	}
	let classes: Vec<String> = methods.iter().map(|(class, _, _)| class.clone()).collect();
	let instances = instance_classes(&node, &classes);
	let returned = returned_classes(&node, &classes);
	let no_parameters = std::collections::HashMap::new();
	let operands = Operands { methods: &methods, instances: &instances, returned: &returned, parameters: &no_parameters };
	let node = specialized_calls(node, &operands);
	let parameters = parameter_classes(&argument_classes(&node, &operands));
	with_operator_calls(node, &Operands { parameters: &parameters, ..operands }).0
}

/// Per defined function, the operator class each argument of each of its calls gives (None: a plain value or unknown)
type CallClasses = std::collections::HashMap<String, Vec<Vec<Option<String>>>>;

fn argument_classes(node: &Node, operands: &Operands) -> CallClasses {
	let defined: std::collections::HashSet<String> = definition_heads(node).into_iter().map(|items| items[0].drop_meta().name()).collect();
	let mut calls = CallClasses::new();
	collect_argument_classes(node, operands, &defined, &mut calls);
	calls
}

/// The class of each parameter that every call of its function gives an instance of: `f(x) := x * 2; f(5 m ± 1 cm)`
/// (card plus-minus-units); None at a position where the calls differ or give no instance
fn parameter_classes(calls: &CallClasses) -> std::collections::HashMap<String, Vec<Option<String>>> {
	let agreed = |calls: &Vec<Vec<Option<String>>>| calls[1..].iter().fold(calls[0].clone(), |known, given| {
		known.iter().zip(given).map(|(known, given)| known.clone().filter(|known| Some(known) == given.as_ref())).collect()
	});
	calls.iter().map(|(function, calls)| (function.clone(), agreed(calls))).collect()
}

/// `f(x) := x * 2` called with a quantity and with a plain number (card mixed-arguments): each call whose argument is
/// an instance where other calls give something else calls a copy typed by that class, `f_Quantity(x:Quantity)`,
/// so the operators in it are the class's methods and the plain calls keep f. Only the program's own functions with
/// untyped parameters are copied, not methods
fn specialized_calls(node: Node, operands: &Operands) -> Node {
	let calls = argument_classes(&node, operands);
	let copyable = copyable_functions(&node);
	let mixed: std::collections::HashMap<String, Vec<bool>> = calls.iter().filter(|(function, _)| copyable.contains(*function)).filter_map(|(function, calls)| {
		let positions = (0..calls[0].len()).map(|position| {
			let given: Vec<&Option<String>> = calls.iter().filter_map(|call| call.get(position)).collect();
			given.iter().any(|class| class.is_some()) && given.iter().any(|class| *class != given[0])
		}).collect::<Vec<bool>>();
		positions.contains(&true).then(|| (function.clone(), positions))
	}).collect();
	if mixed.is_empty() {
		return node;
	}
	let specializations = Specializations { mixed, operands };
	let variants = specializations.variants(&calls);
	specializations.with_copies(specializations.rewrite(node), &variants)
}

/// The functions defined at the top of the program whose parameters are all plain names
fn copyable_functions(node: &Node) -> std::collections::HashSet<String> {
	let statements = match node.drop_meta() {
		Node::List(items, Bracket::None, _) => items.iter().collect(),
		single => vec![single],
	};
	statements.into_iter().filter_map(|statement| match statement.drop_meta() {
		Node::Key(head, Op::Define, _) => match head.drop_meta() {
			Node::List(items, Bracket::Round, _) if items.len() > 1 && items[1..].iter().all(|item| matches!(item.drop_meta(), Node::Symbol(_))) => Some(items[0].drop_meta().name()),
			_ => None,
		},
		_ => None,
	}).collect()
}

struct Specializations<'a> {
	/// Per function, the positions whose calls give different classes
	mixed: std::collections::HashMap<String, Vec<bool>>,
	operands: &'a Operands<'a>,
}

impl Specializations<'_> {
	/// The classes a call gives at its function's mixed positions, when it gives one there
	fn variant(&self, function: &str, given: &[Option<String>]) -> Option<Vec<Option<String>>> {
		let mixed = self.mixed.get(function)?;
		let variant: Vec<Option<String>> = given.iter().zip(mixed).map(|(class, mixed)| class.clone().filter(|_| *mixed)).collect();
		variant.iter().any(Option::is_some).then_some(variant)
	}

	fn variants(&self, calls: &CallClasses) -> std::collections::HashMap<String, std::collections::BTreeSet<Vec<Option<String>>>> {
		let mut variants: std::collections::HashMap<String, std::collections::BTreeSet<Vec<Option<String>>>> = std::collections::HashMap::new();
		for (function, calls) in calls {
			variants.entry(function.clone()).or_default().extend(calls.iter().filter_map(|given| self.variant(function, given)));
		}
		variants
	}

	fn rewrite(&self, node: Node) -> Node {
		match node {
			Node::List(items, Bracket::Round, separator) if items.len() > 1 && self.mixed.contains_key(&items[0].drop_meta().name()) => {
				let function = items[0].drop_meta().name();
				let given: Vec<Option<String>> = items[1..].iter().map(|argument| argument_class(argument, self.operands)).collect();
				let head = match self.variant(&function, &given) {
					Some(variant) => Node::Symbol(variant_name(&function, &variant)),
					None => items[0].clone(),
				};
				let arguments = items[1..].iter().map(|argument| self.rewrite(argument.clone()));
				Node::List(std::iter::once(head).chain(arguments).collect(), Bracket::Round, separator)
			}
			other => other.map_children(|child| self.rewrite(child)),
		}
	}

	/// Each definition of a function called with instances, followed by its typed copies
	fn with_copies(&self, node: Node, variants: &std::collections::HashMap<String, std::collections::BTreeSet<Vec<Option<String>>>>) -> Node {
		match node {
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.with_copies(*node, variants)), data },
			Node::List(items, Bracket::None, separator) => {
				let copies = |item: &Node| {
					let Node::Key(head, Op::Define, body) = item.drop_meta() else { return vec![] };
					let Node::List(parameters, Bracket::Round, head_separator) = head.drop_meta() else { return vec![] };
					let function = parameters[0].drop_meta().name();
					variants.get(&function).into_iter().flatten().map(|variant| {
						let typed = parameters[1..].iter().zip(variant).map(|(parameter, class)| match class {
							Some(class) => key(parameter.clone(), Op::Colon, Node::Symbol(class.clone())),
							None => parameter.clone(),
						});
						let head = Node::List(std::iter::once(Node::Symbol(variant_name(&function, variant))).chain(typed).collect(), Bracket::Round, head_separator.clone());
						Node::Key(Box::new(head), Op::Define, body.clone())
					}).collect()
				};
				let items = items.into_iter().flat_map(|item| { let copies = copies(&item); std::iter::once(item).chain(copies) }).collect();
				Node::List(items, Bracket::None, separator)
			}
			other => other,
		}
	}
}

/// `f_Quantity`, the copy of f for its classes at the mixed positions
fn variant_name(function: &str, variant: &[Option<String>]) -> String {
	std::iter::once(function).chain(variant.iter().flatten().map(String::as_str)).collect::<Vec<_>>().join("_")
}

fn definition_heads(node: &Node) -> Vec<Vec<Node>> {
	let mut heads = vec![];
	node.visit(&mut |part| if let Node::Key(head, Op::Define, _) = part {
		if let Node::List(items, Bracket::Round, _) = head.drop_meta() {
			if items.len() > 1 {
				heads.push(items.clone());
			}
		}
	});
	heads
}

fn collect_argument_classes(node: &Node, operands: &Operands, defined: &std::collections::HashSet<String>, calls: &mut CallClasses) {
	match node.drop_meta() {
		Node::Key(_, Op::Define, body) => collect_argument_classes(body, operands, defined, calls),
		Node::List(items, Bracket::Round, _) if items.len() > 1 && defined.contains(&items[0].drop_meta().name()) => {
			let given: Vec<Option<String>> = items[1..].iter().map(|argument| argument_class(argument, operands)).collect();
			calls.entry(items[0].drop_meta().name()).or_default().push(given);
			items[1..].iter().for_each(|argument| collect_argument_classes(argument, operands, defined, calls));
		}
		other => children_of(other).into_iter().for_each(|child| collect_argument_classes(child, operands, defined, calls)),
	}
}

/// The class of an argument that is an instance of a class with operator methods
fn argument_class(argument: &Node, operands: &Operands) -> Option<String> {
	operand_class(argument, operands).filter(|class| operands.methods.iter().any(|(owner, _, _)| owner == class))
}

fn children_of(node: &Node) -> Vec<&Node> {
	match node {
		Node::Key(left, _, right) => vec![left, right],
		Node::List(items, _, _) => items.iter().collect(),
		_ => vec![],
	}
}

/// What tells an operator's operand an instance: the classes' operator methods, the variables holding instances, the
/// functions returning them
struct Operands<'a> {
	methods: &'a [(String, Op, String)],
	instances: &'a std::collections::HashMap<String, String>,
	returned: &'a std::collections::HashMap<String, String>,
	/// parameter_classes
	parameters: &'a std::collections::HashMap<String, Vec<Option<String>>>,
}

/// The node with its operator calls, and the class of the instance it gives when it is one
fn with_operator_calls(node: Node, operands: &Operands) -> (Node, Option<String>) {
	let recurse = |child: Node| with_operator_calls(child, operands).0;
	match node {
		certainty if crate::uncertain::certainty_parts(&certainty).is_some() => (with_certain_amounts(&certainty, operands), None),
		sum if summed_quantities(&sum, operands).is_some() => {
			let quantities = with_operator_calls(summed_quantities(&sum, operands).expect("guarded").clone(), operands).0;
			(quantities_reduced(quantities), Some(crate::units::RUN_TIME_QUANTITY.to_string()))
		}
		Node::Key(left, op, right) if OPERATOR_METHODS.iter().any(|(known, _)| *known == op) => {
			let (left, class) = with_operator_calls(*left, operands);
			let (right, right_class) = with_operator_calls(*right, operands);
			let class = class.or_else(|| operand_class(&left, operands));
			let right_class = right_class.or_else(|| operand_class(&right, operands));
			// `"v: " + q` of a run-time quantity q joins its text, as a static quantity does (card quantity-falls)
			if op == Op::Add && (is_text(&left) || is_text(&right)) {
				return (key(joined_text(left, class), op, joined_text(right, right_class)), None);
			}
			let (left, class, right) = with_run_time_units(left, class, right, right_class, operands);
			match class.as_ref().and_then(|class| operands.methods.iter().find(|(owner, known, _)| owner == class && *known == op)) {
				Some((_, _, method)) => (method_call(left, method, right), class),
				None => (key(left, op, right), None),
			}
		}
		// `q as km/h`, `q in m` of a run-time quantity q: its conversion `q.to("km/h")` (card quantity-falls)
		conversion if crate::units::conversion(&conversion).is_some() => {
			let (quantity, units) = crate::units::conversion(&conversion).map(|(quantity, units)| (quantity.clone(), units)).expect("guarded");
			let (quantity, class) = with_operator_calls(quantity, operands);
			let run_time = Some(crate::units::RUN_TIME_QUANTITY);
			match class.or_else(|| operand_class(&quantity, operands)).as_deref() == run_time {
				true => (method_call(quantity, CONVERSION_METHOD, Node::Text(crate::units::unit_suffix(&units))), run_time.map(str::to_string)),
				false => (conversion.map_children(recurse), None),
			}
		}
		// `1 min == q` of a run-time quantity q (the equality witness compares them)
		Node::Key(left, op @ (Op::Eq | Op::Ne), right) => {
			let (left, class) = with_operator_calls(*left, operands);
			let (right, right_class) = with_operator_calls(*right, operands);
			let class = class.or_else(|| operand_class(&left, operands));
			let (left, _, right) = with_run_time_units(left, class, right, right_class, operands);
			(key(left, op, right), None)
		}
		Node::List(items, Bracket::Round, separator) if items.len() == 1 => {
			let (item, class) = with_operator_calls(items[0].clone(), operands);
			(Node::List(vec![item], Bracket::Round, separator), class)
		}
		// `f(a) := a + 1`: its parameters are none of the program's instance variables of the same names; one typed by a
		// class (`q:Quantity`), or given instances of one class by every call, is an instance of it
		Node::Key(head, Op::Define, body) if matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() > 1) => {
			let Node::List(items, _, _) = head.drop_meta() else { unreachable!("guarded") };
			let mut instances = operands.instances.clone();
			let called_with = operands.parameters.get(&items[0].drop_meta().name());
			for (position, parameter) in items[1..].iter().enumerate() {
				let (name, declared) = match parameter.drop_meta() {
					Node::Key(name, Op::Colon, type_node) => (name.drop_meta().name(), Some(type_node.drop_meta().name())),
					other => (other.name(), called_with.and_then(|classes| classes.get(position).cloned().flatten())),
				};
				match declared.filter(|class| operands.methods.iter().any(|(owner, _, _)| owner == class)) {
					Some(class) => instances.insert(name, class),
					None => instances.remove(&name),
				};
			}
			let body = with_operator_calls(*body, &Operands { instances: &instances, ..*operands }).0;
			(Node::Key(head, Op::Define, Box::new(body)), None)
		}
		other => (other.map_children(recurse), None),
	}
}

/// `receiver.method(argument)`; `(quantity(…)) * 2`: the receiver without its parentheses, else `(f(…)).times` reads as
/// a call of f
/// `rope certainly > 4 m` of a run-time quantity rope: its amount and the other's in the same base units compare, a ±
/// amount as the interval it is; `Quantity.more` would answer a plain yes (card quantity-tolerance)
fn with_certain_amounts(certainty: &Node, operands: &Operands) -> Node {
	let (word, ordering) = crate::uncertain::certainty_parts(certainty).expect("guarded");
	let Node::Key(left, op, right) = ordering.drop_meta() else { unreachable!("certainty_parts takes orderings") };
	let (left, class) = with_operator_calls(left.as_ref().clone(), operands);
	let (right, right_class) = with_operator_calls(right.as_ref().clone(), operands);
	let class = class.or_else(|| operand_class(&left, operands));
	let (left, class, right) = with_run_time_units(left, class, right, right_class, operands);
	let compared = match class.as_deref() == Some(crate::units::RUN_TIME_QUANTITY) {
		true => {
			let comparable = call(SAME_DIMENSION, vec![left.clone(), right, text("compare")]);
			key(amount_of(left), *op, amount_of(comparable))
		}
		false => key(left, *op, right),
	};
	Node::List(vec![symbol(word), compared], Bracket::None, Separator::Space).with_meta_of(certainty)
}

/// The list of `sum([5 m ± 1 cm, 3 m ± 2 cm])` or `[…].sum()` when each item is a run-time quantity
fn summed_quantities<'n>(node: &'n Node, operands: &Operands) -> Option<&'n Node> {
	let list = summed_list(node)?;
	let Node::List(items, Bracket::Square, _) = list.drop_meta() else { return None };
	let is_quantity = |item: &Node| operand_class(item, operands).as_deref() == Some(crate::units::RUN_TIME_QUANTITY);
	(!items.is_empty() && items.iter().all(is_quantity)).then_some(list)
}

/// The list of `sum(xs)`, `xs.sum()`
fn summed_list(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) => match items.as_slice() {
			[word, list] if word.drop_meta().name() == SUM_WORD => Some(list),
			_ => None,
		},
		Node::Key(list, Op::Dot, call) if is_call_of(call, SUM_WORD) || call.drop_meta().name() == SUM_WORD => Some(list),
		_ => None,
	}
}

/// `quantities.reduce((sum, item) => sum.plus(item))`, marked a Quantity so `str()` and a final value show its text
/// (card quantity-sum)
fn quantities_reduced(quantities: Node) -> Node {
	let [sum, item] = REDUCED_NAMES.map(symbol);
	let parameters = Node::List(vec![sum.clone(), item.clone()], Bracket::Round, Separator::Colon);
	let added = key(parameters, Op::FatArrow, method_call(sum, operator_method(Op::Add).expect("plus"), item));
	let reduced = key(quantities, Op::Dot, call(REDUCE_WORD, vec![added]));
	Node::meta(reduced, Node::data(crate::lowering::traits::TypedAs(crate::units::RUN_TIME_QUANTITY.to_string())))
}

fn amount_of(quantity: Node) -> Node {
	key(quantity, Op::Dot, symbol(QUANTITY_AMOUNT))
}

fn method_call(receiver: Node, method: &str, argument: Node) -> Node {
	let receiver = match receiver.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => items[0].clone(),
		_ => receiver,
	};
	let call = call(method, vec![argument]);
	key(receiver, Op::Dot, call)
}

/// `2 km < q`, `q + 1 m` of a run-time quantity q: the unit written in the program is one too (card units-mixed); the
/// operands with the left one's class
fn with_run_time_units(left: Node, class: Option<String>, right: Node, right_class: Option<String>, operands: &Operands) -> (Node, Option<String>, Node) {
	let run_time = Some(crate::units::RUN_TIME_QUANTITY);
	let lifted_left = class.is_none() && right_class.or_else(|| operand_class(&right, operands)).as_deref() == run_time;
	let (left, class) = match lifted_left.then(|| crate::units::as_run_time_quantity(&left)).flatten() {
		Some(quantity) => (quantity, run_time.map(str::to_string)),
		None => (left, class),
	};
	let right = match class.as_deref() == run_time {
		true => crate::units::as_run_time_quantity(&right).unwrap_or(right),
		false => right,
	};
	(left, class, right)
}

/// The class of the instance an operand is: a variable holding one, a parenthesized operation, a construction or a call
/// of a function returning one
fn operand_class(operand: &Node, operands: &Operands) -> Option<String> {
	match operand.drop_meta() {
		Node::Symbol(variable) => operands.instances.get(variable).cloned(),
		// `(a + b).x`: the parenthesized operation, `(quantity("5 m")) * 2` the parenthesized instance
		Node::List(items, Bracket::Round, _) if items.len() == 1 => with_operator_calls(items[0].clone(), operands).1.or_else(|| operand_class(&items[0], operands)),
		// `f(quantity("5 m") / 5)`: an operation of an instance as an argument
		Node::Key(_, op, _) if OPERATOR_METHODS.iter().any(|(known, _)| known == op) => with_operator_calls(operand.drop_meta().clone(), operands).1,
		// `q.times(2)`: an operator method already called on an instance gives one of its class
		Node::Key(receiver, Op::Dot, call) if is_operator_method_call(call, operands) || is_call_of(call, CONVERSION_METHOD) => operand_class(receiver, operands),
		other => instance_class(other, operands.returned),
	}
}

fn is_text(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Text(_))
}

/// An operand joining a text: a run-time quantity as `str(q)`
fn joined_text(operand: Node, class: Option<String>) -> Node {
	match class.as_deref() == Some(crate::units::RUN_TIME_QUANTITY) {
		true => call(TEXT_WORD, vec![operand]),
		false => operand,
	}
}

fn is_operator_method_call(call: &Node, operands: &Operands) -> bool {
	operands.methods.iter().any(|(_, _, method)| is_call_of(call, method))
}

/// `to("m")` is a call of `to`
fn is_call_of(call: &Node, method: &str) -> bool {
	matches!(call.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().is_some_and(|word| word.drop_meta().name() == method))
}

/// A name the library gives lists and texts too (`pop`, `sum`, `count`): a method of that name is one only on an
/// instance
fn is_library_method(name: &str) -> bool {
	crate::analyzer::is_list_mutating_method(name) || crate::library_words::is_library_word(name) || crate::analyzer::is_counting_property(name)
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
		_ => return go_method(words),
	};
	let Node::Symbol(class) = class.drop_meta() else { return None };
	match block.drop_meta() {
		Node::List(_, Bracket::Curly, _) => Some((class.clone(), class_items(block))),
		_ => None,
	}
}

/// Go's method `func (p Point) Sum() int {…}` (or of the pointer `(p *Point)`): the class and the method `func Sum() int
/// {…}`, its receiver p read as self
fn go_method(words: &[Node]) -> Option<(String, Vec<Node>)> {
	let [keyword, receiver, rest @ ..] = words else { return None };
	if keyword.drop_meta().name() != GO_FUNCTION_WORD || rest.is_empty() {
		return None;
	}
	let Node::List(receiver, Bracket::Round, _) = receiver.drop_meta() else { return None };
	let (variable, class) = match receiver.as_slice() {
		[variable, class] => (variable.drop_meta(), class.drop_meta()),
		[single] => match single.drop_meta() {
			Node::Key(variable, Op::Mul, class) => (variable.drop_meta(), class.drop_meta()),
			_ => return None,
		},
		_ => return None,
	};
	let class = match class {
		Node::Key(empty, Op::Mul, class) if matches!(empty.drop_meta(), Node::Empty) => class.drop_meta(),
		class => class,
	};
	let (Node::Symbol(variable), Node::Symbol(class)) = (variable, class) else { return None };
	let rest: Vec<Node> = match rest {
		[Node::List(group, Bracket::None, _)] => group.clone(),
		_ => rest.to_vec(),
	};
	let bindings = [(variable.clone(), symbol(RECEIVER))].into_iter().collect();
	let method = crate::law::substitute(&Node::List([vec![keyword.clone()], rest].concat(), Bracket::None, Separator::Space), &bindings);
	Some((class.clone(), class_items(&Node::List(vec![method], Bracket::Curly, Separator::Semicolon))))
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

/// Every class with the items of the mixins it takes in (`class Duck with Walker {…}`, `include Walker` in its body)
/// after its own, but those it defines itself; the mixin declarations themselves are no classes and go
fn with_mixins(node: Node) -> Result<Node, Node> {
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

fn taking_in_mixins(node: Node, mixins: &[(String, Vec<Node>)]) -> Result<Node, Node> {
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
	let is_modifier = |word: &Node| matches!(word.drop_meta(), Node::Symbol(word) if crate::warp_parser::MEMBER_MODIFIERS.contains(&word.as_str()) || FIELD_KEYWORDS.contains(&word.as_str()));
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

/// Ruby's `Point.new(1, 2)` of a declared class that defines no `new`: the construction `Point(1, 2)`, with a note
fn ruby_constructions(node: Node) -> Node {
	let mut classes = vec![];
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		if !class_items(body).iter().filter_map(method_parts).any(|(method, _, _)| method == RUBY_NEW_WORD) {
			classes.push(name.drop_meta().name());
		}
	});
	if classes.is_empty() {
		return node;
	}
	constructed_by_new(node, &classes)
}

/// Ruby's `Point.new(1, 2)` and Java's `new Set(xs)` of a class the parser did not see (a standard module's): the
/// construction, with a note
fn constructed_by_new(node: Node, classes: &[String]) -> Node {
	match node {
		Node::List(items, Bracket::None, Separator::Space) if matches!(items.as_slice(), [new, call] if new.drop_meta().name() == RUBY_NEW_WORD && classes.contains(&leading_name(call))) => {
			let call = items[1].clone();
			crate::diagnostic::note_alias(&format!("{RUBY_NEW_WORD} {}", leading_name(&call)), &leading_name(&call));
			constructed_by_new(call, classes)
		}
		Node::Key(class, Op::Dot, member) if classes.contains(&class.drop_meta().name()) && matches!(class.drop_meta(), Node::Symbol(_)) && leading_name(&member) == RUBY_NEW_WORD => {
			let class_name = class.drop_meta().name();
			crate::normalize::set_position_of(&class);
			crate::diagnostic::note_alias(&format!("{class_name}.{RUBY_NEW_WORD}"), &class_name);
			let arguments = match member.drop_meta().clone() {
				Node::List(items, _, _) => items[1..].iter().cloned().map(|argument| constructed_by_new(argument, classes)).collect(),
				_ => vec![],
			};
			Node::List([vec![*class], arguments].concat(), Bracket::Round, Separator::None)
		}
		other => other.map_children(|child| constructed_by_new(child, classes)),
	}
}

/// Go's positional braces `Point{1, 2}` of a declared class: the construction `Point(1, 2)`, with a note (P167); of
/// an unknown name they stay tagged data
fn positional_braces(node: Node) -> Node {
	let mut classes = vec![];
	node.visit(&mut |part| if let Node::Type { name, .. } = part {
		classes.push(name.drop_meta().name());
	});
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

fn is_witness_method(name: &str) -> bool {
	WITNESS_METHODS.iter().any(|(witness, _)| *witness == name)
}

/// A class's `text()`, `equals(o)` and `compare(o)` (Java's `toString()`, `compareTo(o)`, Python's `__str__`, `__eq__`…,
/// with a note) as the witnesses of its text, equality and order: the other instance typed by the class,
/// `equals(o:P)`, so the method is `equals(self:P, o:P)`, which traits makes equals·P
fn with_witness_methods(node: Node) -> Node {
	match node {
		Node::Type { name, body } => {
			let class = name.drop_meta().name();
			let items: Vec<Node> = class_items(&body);
			if !items.iter().filter_map(method_parts).any(|(method, _, _)| WITNESS_METHODS.iter().any(|(witness, aliases)| *witness == method || aliases.contains(&method.as_str()))) {
				return Node::Type { name, body };
			}
			let items = items.into_iter().map(|item| witness_method(item, &class)).collect();
			Node::Type { name, body: Box::new(Node::List(items, Bracket::Curly, Separator::Semicolon)) }
		}
		other => other.map_children(with_witness_methods),
	}
}

fn witness_method(item: Node, class: &str) -> Node {
	let Some((method, parameters, _)) = method_parts(&item) else { return item };
	let Some((witness, _)) = WITNESS_METHODS.iter().find(|(witness, aliases)| *witness == method || aliases.contains(&method.as_str())) else { return item };
	if *witness != method {
		crate::diagnostic::note_alias(&method, witness);
	}
	let typed = |parameter: Node| match parameter.drop_meta() {
		Node::Symbol(_) if *witness != "text" => key(parameter, Op::Colon, symbol(class)),
		_ => parameter,
	};
	let head = call(witness, parameters.into_iter().map(typed).collect());
	match item.drop_meta().clone() {
		Node::Key(old_head, Op::Define, body) => match old_head.drop_meta() {
			Node::Key(_, Op::Colon, result) if result_type(&item).is_some() => Node::Key(Box::new(Node::Key(Box::new(head), Op::Colon, result.clone())), Op::Define, body),
			_ => Node::Key(Box::new(head), Op::Define, body),
		},
		_ => item,
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
		Node::List(parts, bracket, separator) => Node::List([vec![symbol(name)], parts[1..].to_vec()].concat(), bracket.clone(), separator.clone()),
		_ => symbol(name),
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
fn method_calls(node: Node, called: &[String], changing: &Changing, calls: &std::cell::Cell<usize>) -> Node {
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
			calls.set(calls.get() + 1);
			let pair = Node::Symbol(format!("{name}{RESULT_SUFFIX}·{}", calls.get()));
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
	let declaration = key(symbol(GLOBAL_KEYWORD), Op::Colon, declaration);
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
	let is_return = |word: &Node| matches!(word.drop_meta(), Node::Symbol(name) if name == RETURN_WORD);
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
