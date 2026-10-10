//! `P(1, 2)` for a declared type `type P{x:int y:int}` constructs the instance `P{x:1 y:2}`.
//! `P{x:1 y:2}` (glued, parsed as the key `P` Op::None `{…}`) constructs and validates one too, while `P:{…}` is plain
//! data (D4): an instance is the data key marked `Instance`, emitted with its own op code, so it never equals the data.

use super::nodes::key;
use crate::analyzer::{builtin_type_kind, call_name, collect_all_types, list_element_type, literal_misfit, misfit_item};
use crate::diagnostic::Diagnostic;
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::{Kind, TypeDef, TypeRegistry};

/// Marks the data key `P:{…}` that a construction `P{…}` validated
#[derive(Clone, Debug, PartialEq)]
pub struct Instance;

/// The type name and fields of a constructed instance, behind any metadata
pub fn instance_parts(node: &Node) -> Option<(&Node, &Node)> {
	match node {
		Node::Meta { node, data } if matches!(data.as_ref(), Node::Data(dada) if dada.downcast_ref::<Instance>().is_some()) => match node.drop_meta() {
			Node::Key(name, Op::Colon, fields) => Some((name, fields)),
			_ => None,
		},
		Node::Meta { node, .. } => instance_parts(node),
		_ => None,
	}
}

/// Does the node carry the Instance mark: a construction, or the annotation of an instance parameter `p:person`
pub fn instance_parts_marked(node: &Node) -> bool {
	match node {
		Node::Meta { node, data } => matches!(data.as_ref(), Node::Data(dada) if dada.downcast_ref::<Instance>().is_some()) || instance_parts_marked(node),
		_ => false,
	}
}

pub fn lower(node: Node) -> Node {
	let mut registry = TypeRegistry::new();
	collect_all_types(&mut registry, &node);
	let constructors = defined_constructors(&node, &registry);
	construct(node, &Classes { registry: &registry, constructors: &constructors, defaulting: Default::default() })
}

/// The declared types and their `init{…}` / `init(name){…}` constructors (class_methods::constructor_name), each
/// with the number of parameters it takes besides the instance
struct Classes<'a> {
	registry: &'a TypeRegistry,
	constructors: &'a [(String, usize)],
	/// The types whose field defaults are being constructed: a default constructing its own type stays as written
	defaulting: std::cell::RefCell<Vec<String>>,
}

impl Classes<'_> {
	/// A field's declared default, constructed as any other value: `p: Point = Point(0)` is a Point, made anew by each
	/// construction that leaves p out, so no two instances share it (P200)
	fn default_of(&self, type_def: &TypeDef, field: &crate::type_kinds::FieldDef) -> Option<Node> {
		let value = self.registry.default_of(type_def, field)?.clone();
		if self.defaulting.borrow().contains(&type_def.name) {
			return Some(value);
		}
		self.defaulting.borrow_mut().push(type_def.name.clone());
		let constructed = construct(value, self);
		self.defaulting.borrow_mut().pop();
		Some(constructed)
	}

	fn constructor(&self, class: &str, parameters: usize) -> Option<Node> {
		let name = crate::class_methods::constructor_name(class);
		self.constructors.iter().any(|(defined, count)| *defined == name && *count == parameters).then_some(Node::Symbol(name))
	}

	/// The instance passed through its class's constructor without parameters, when the class has one
	fn constructed(&self, class: &str, instance: Node) -> Node {
		match self.constructor(class, 0) {
			Some(constructor) => Node::List(vec![constructor, instance], Bracket::Round, Separator::None),
			None => instance,
		}
	}

	/// `P(a, b)` of a class whose constructor `init(x, y){…}` takes these arguments: the instance of the declared
	/// defaults (ø for the others) passed through it with them
	fn constructed_by_parameters(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		let class = call_name(items, bracket, separator)?;
		let constructor = self.constructor(class, items.len() - 1)?;
		let type_def = self.registry.get_by_name(class)?;
		let fields = type_def.fields.iter().map(|field| {
			let value = self.default_of(type_def, field).unwrap_or_else(|| unset_field(field));
			key(Node::Symbol(field.name.clone()), Op::Colon, value)
		}).collect();
		let arguments = std::iter::once(constructor).chain(std::iter::once(instance_node(class, fields))).chain(items[1..].iter().cloned());
		Some(Node::List(arguments.collect(), Bracket::Round, Separator::None))
	}
}

/// A field the constructor sets, before it does: an Int or float field (a raw number in the struct) is 0, any other ø
fn unset_field(field: &crate::type_kinds::FieldDef) -> Node {
	use crate::type_kinds::Kind;
	crate::analyzer::type_word_kind(&field.type_name)
		.filter(|kind| matches!(kind, Kind::Int | Kind::Float))
		.and_then(crate::declarations::zero_value)
		.unwrap_or(Node::Empty)
}

/// The constructor functions the program defines, with the number of their parameters besides the instance
fn defined_constructors(node: &Node, registry: &TypeRegistry) -> Vec<(String, usize)> {
	let names: Vec<String> = registry.types().iter().map(|type_def| crate::class_methods::constructor_name(&type_def.name)).collect();
	let mut defined = vec![];
	node.visit(&mut |part| if let Node::Key(head, Op::Define, _) = part {
		if let Node::List(items, Bracket::Round, _) = head.drop_meta() {
			let name = items.first().map(|first| first.drop_meta().name()).unwrap_or_default();
			if names.contains(&name) {
				defined.push((name, items.len().saturating_sub(2)));
			}
		}
	});
	defined
}

fn construct(node: Node, classes: &Classes) -> Node {
	let registry = classes.registry;
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| construct(item, classes)).collect();
			if let Some(constructed) = classes.constructed_by_parameters(&items, &bracket, &separator) {
				return constructed;
			}
			instance(&items, &bracket, &separator, classes).map(|instance| match call_name(&items, &bracket, &separator) {
				Some(class) => classes.constructed(class, instance),
				None => instance,
			}).unwrap_or(Node::List(items, bracket, separator))
		}
		Node::Key(name, Op::None, fields) if matches!(name.drop_meta(), Node::Symbol(_)) => {
			let entries = entries(&construct(*fields, classes));
			match registry.get_by_name(&name.name()) {
				Some(type_def) => match field_error(type_def, registry, &name, &entries) {
					Some(error) => error,
					None => {
						let fields = Node::List(with_defaults(type_def, classes, entries), Bracket::Curly, Separator::Space);
						let class = name.drop_meta().name();
						classes.constructed(&class, Node::meta(Node::Key(name, Op::Colon, Box::new(fields)), Node::data(Instance)))
					}
				},
				// the pre-scan of the parser can mistake a word for a type name: plain data
				None => Node::Key(name, Op::Colon, Box::new(Node::List(entries, Bracket::Curly, Separator::Space))),
			}
		}
		Node::Key(left, op, right) => key(construct(*left, classes), op, construct(*right, classes)),
		Node::Meta { node, data } => Node::Meta { node: Box::new(construct(*node, classes)), data },
		other => other,
	}
}

/// The instance a call of a declared type constructs, `P(1, 2)`, `P(x:1, y:2)` or `P(1, y:2)`; fields are checked as
/// the braces `P{…}` check them; a wrong argument count is an error value at the call
fn instance(items: &[Node], bracket: &Bracket, separator: &Separator, classes: &Classes) -> Option<Node> {
	let registry = classes.registry;
	let name = call_name(items, bracket, separator)?;
	let type_def = registry.get_by_name(name)?;
	let arguments = &items[1..];
	let (positional, named): (Vec<&Node>, Vec<&Node>) = arguments.iter().partition(|argument| !is_named(argument, type_def));
	let required = type_def.fields.iter().filter(|field| registry.is_required(type_def, field)).count();
	if named.is_empty() && (arguments.len() < required || arguments.len() > type_def.fields.len()) {
		let message = if required == type_def.fields.len() {
			format!("{name} takes {} fields, got {}", type_def.fields.len(), arguments.len())
		} else {
			format!("{name} takes {required} to {} fields, got {}", type_def.fields.len(), arguments.len())
		};
		return Some(Diagnostic::at(&items[0], message).into_error());
	}
	let entry = |field: &str, value: &Node| key(symbol(field), Op::Colon, value.clone());
	let given: Vec<Node> = type_def.fields.iter().zip(&positional).map(|(field, value)| entry(&field.name, value))
		.chain(named.into_iter().cloned())
		.collect();
	if let Some(error) = field_error(type_def, registry, &items[0], &given) {
		return Some(error);
	}
	let fields = type_def.fields.iter().map(|field| {
		let value = given.iter().find(|entry| entry_name(entry).as_ref() == Some(&field.name)).map(entry_value)
			.cloned().or_else(|| classes.default_of(type_def, field)).unwrap_or(Node::Empty);
		entry(&field.name, &value)
	}).collect();
	Some(instance_node(name, fields))
}

/// `x:1` or `x=1` (as a function's named argument, P37) among the arguments of a construction: the declared field x
/// given by name; an instance `engine(90)` is a value
fn is_named(argument: &Node, type_def: &TypeDef) -> bool {
	let names_field = |field: &Node| matches!(field.drop_meta(), Node::Symbol(name) if type_def.fields.iter().any(|declared| declared.name == *name));
	// `z=2` names a field even where P has none: "P has no field z", as a function's "f has no parameter z"
	!instance_parts_marked(argument) && match argument.drop_meta() {
		Node::Key(field, Op::Colon, _) => names_field(field),
		Node::Key(field, Op::Assign, _) => matches!(field.drop_meta(), Node::Symbol(_)),
		_ => false,
	}
}

pub(crate) fn entry_value(entry: &Node) -> &Node {
	match entry.drop_meta() {
		Node::Key(_, _, value) => value,
		other => other,
	}
}

/// The instance of the type `name` with these field entries
pub fn instance_node(name: &str, entries: Vec<Node>) -> Node {
	let body = Node::List(entries, Bracket::Curly, Separator::Space);
	Node::meta(key(symbol(name), Op::Colon, body), Node::data(Instance))
}

fn entries(fields: &Node) -> Vec<Node> {
	match fields.drop_meta() {
		Node::List(items, _, _) => items.clone(),
		Node::Empty => vec![],
		single => vec![single.clone()],
	}
}

pub(crate) fn entry_name(entry: &Node) -> Option<String> {
	match entry.drop_meta() {
		Node::Key(field, Op::Colon | Op::Assign, _) => Some(field.name()),
		_ => None,
	}
}

/// The given fields, then every left out field that has a default, with its default value, and every left out optional one
fn with_defaults(type_def: &TypeDef, classes: &Classes, mut entries: Vec<Node>) -> Vec<Node> {
	let given: Vec<String> = entries.iter().filter_map(entry_name).collect();
	for field in type_def.fields.iter().filter(|field| !given.contains(&field.name)) {
		// a left out optional field `left?` is there, holding ø: reading it is no error, assigning it changes it
		let value = classes.default_of(type_def, field).or_else(|| field.is_optional().then_some(Node::Empty));
		if let Some(value) = value {
			entries.push(key(Node::Symbol(field.name.clone()), Op::Colon, value));
		}
	}
	entries
}

/// Strict construction (user, D4): every field of `P{…}` is declared, fits its declared type, and every required field
/// (neither `x?` nor with a default) is given
fn field_error(type_def: &TypeDef, registry: &TypeRegistry, located: &Node, entries: &[Node]) -> Option<Node> {
	let name = &type_def.name;
	// a meta entry `@source:"gps"` is never a field
	let wrong_entry = entries.iter().filter(|entry| crate::node::meta_entry(entry).is_none()).find_map(|entry| {
		let Node::Key(field, Op::Colon | Op::Assign, value) = entry.drop_meta() else {
			let message = format!("{name}{{…}} takes fields as name:value, got {}", entry.serialize());
			return Some(Diagnostic::at(located, message).into_error());
		};
		let field = field.name();
		let Some(declared) = type_def.fields.iter().find(|declared| declared.name == field) else {
			let message = format!("{name} has no field {field}");
			return Some(Diagnostic::at(located, message).fix(format!("declare {field} in {name}, or write the data {name}:{{…}}")).into_error());
		};
		let declared_type = declared.type_name.trim_end_matches('?');
		// a list field `items: texts` takes a literal list of fitting items (card list-element-types)
		if let Some(element) = list_element_type(declared_type) {
			let (item, actual) = misfit_item(element, value)?;
			let message = format!("{name}.{field} is {declared_type}, got the {} item {}", format!("{actual:?}").to_lowercase(), item.serialize());
			return Some(Diagnostic::at(located, message).into_error());
		}
		let actual = literal_misfit(declared_type, value)?;
		let mut message = format!("{name}.{field} is {declared_type}, got {} {}", format!("{actual:?}").to_lowercase(), value.serialize());
		if builtin_type_kind(declared_type) == Some(Kind::Int) && actual == Kind::Float {
			message += ": an int must be a whole number"; // as the run-time check says (list_ops INT_NOT_WHOLE)
		}
		Some(Diagnostic::at(located, message).into_error())
	});
	wrong_entry.or_else(|| {
		let given: Vec<String> = entries.iter().filter_map(entry_name).collect();
		let missing = type_def.fields.iter().find(|field| registry.is_required(type_def, field) && !given.contains(&field.name))?;
		let message = format!("{name} needs field {}", missing.name);
		Some(Diagnostic::at(located, message).fix(format!("give {0}:…, or declare it {0}? or with a default", missing.name)).into_error())
	})
}
