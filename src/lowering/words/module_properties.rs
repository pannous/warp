//! Properties of objects (decision property-accessors, card graphics-properties): `window.width`, `display.height`,
//! `canvas.width` read like fields and, where they have a setter, are written like them. A property is defined in
//! warp, its getter `canvas.width := canvas_width` and its setter the one-parameter overload `canvas.width(w) := canvas(w,
//! canvas_height)` (or a getter-shaped body reading `it`); `canvas.width = 300`, `canvas.width(300)` and `+=` and its kin
//! call the setter, writing a property without one is a loud error saying the comment above its getter. The standard
//! ones are lines of lib/draw.warp (the canvas, which comes along) and lib/prelude.warp (window and display, system
//! values, so `whenever window.width > 800 {…}` listens); a program defines its own the same way. A use is the
//! definition's body in place, as a getter's text was. A bare `width` or `height` the program does not bind is the
//! canvas's, with a did-you-mean naming the three. A program binding the object or the word itself
//! (`canvas = {width: 9}`, `f(width) := …`) keeps its own; a field name (`{width: 5}`, `p.width`) is none.
//! Runs before modules::resolve, which then brings lib/draw.warp, and before system_values marks the window's reads.

use super::nodes::{key, parameter_name};
use crate::errors::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

/// The parameter of a setter written getter-shaped: `canvas.width := canvas(it, canvas_height)`
const WRITTEN: &str = "it";
const CANVAS: &str = "canvas";
const SIDES: [&str; 2] = ["width", "height"];
const DENSITY_NOTE: &str = "the pixels per length, in dpi";

/// Other names of a property: object, alias, the property, the note naming it (decision display-names)
const PROPERTY_ALIASES: [(&str, &str, &str, &str); 4] = [
	("display", "dpi", "density", DENSITY_NOTE),
	("display", "ppi", "density", DENSITY_NOTE),
	("display", "dimensions", "size", "dimension is a physical quantity (length, mass); size is width × height in px"),
	("display", "resolution", "size", "resolution: density (dpi) or size (px)? size is width × height"),
];

/// `object.property`: its getter body, its setter (parameter, body), the comment above its getter, the module it needs
#[derive(Clone, Default)]
struct Property {
	getter: Option<Node>,
	setter: Option<(String, Node)>,
	comment: Option<String>,
	module: Option<&'static str>,
}

type Properties = HashMap<(String, String), Property>;

/// `canvas.width := …` (getter, or setter when it reads `it`) and `canvas.width(w) := …` (setter): object, property,
/// and the setter's parameter
fn definition(statement: &Node) -> Option<(String, String, Option<String>, &Node)> {
	let Node::Key(head, Op::Define, body) = statement.drop_meta() else {
		return None;
	};
	let Node::Key(object, Op::Dot, member) = head.drop_meta() else {
		return None;
	};
	let object = object.drop_meta().symbol_name()?.to_string();
	let (property, parameter) = match member.drop_meta() {
		Node::Symbol(property) => {
			(property.clone(), crate::analyzer::takes_implicit_parameter(body).then(|| WRITTEN.to_string()))
		}
		Node::List(items, Bracket::Round, _) if items.len() == 2 => {
			(items[0].drop_meta().symbol_name()?.to_string(), Some(parameter_name(&items[1])?))
		}
		_ => return None,
	};
	Some((object, property, parameter, body))
}

/// The properties `statements` define, merged into `properties`
fn collect(statements: &[Node], module: Option<&'static str>, properties: &mut Properties) {
	for statement in statements {
		let Some((object, property, parameter, body)) = definition(statement) else {
			continue;
		};
		let known = properties.entry((object, property)).or_default();
		known.module = module;
		match parameter {
			Some(parameter) => known.setter = Some((parameter, body.clone())),
			None => {
				known.getter = Some(body.clone());
				known.comment = crate::lowering::doc_strings::leading_comment(statement).map(str::to_string);
			}
		}
	}
}

/// The properties the standard modules define; the prelude's need no module. Parsed once per thread (Nodes stay on theirs)
fn standard_properties() -> Properties {
	thread_local! {
		static PROPERTIES: Properties = {
			let mut properties = Properties::new();
			for (module, source) in crate::modules::STD_MODULES {
				if !source.lines().any(defines_property) {
					continue;
				}
				// parsed without the module's file: a hint about a property points at the program's use (display-names)
				let parsed = crate::normalize::without_hints(|| crate::warp_parser::WarpParser::parse(source));
				let needed = (module != crate::modules::PRELUDE_MODULE).then_some(module);
				collect(&crate::modules::statements(parsed), needed, &mut properties);
			}
			properties
		};
	}
	PROPERTIES.with(Properties::clone)
}

/// The objects the standard modules give properties (canvas, window, display), read off their text: a program naming
/// none of them nor a canvas side needs none of their properties, whose parse cost a quarter of a small compile
fn standard_objects() -> &'static HashSet<String> {
	static OBJECTS: std::sync::OnceLock<HashSet<String>> = std::sync::OnceLock::new();
	OBJECTS.get_or_init(|| {
		let lines = crate::modules::STD_MODULES.iter().flat_map(|(_, source)| source.lines());
		lines
			.filter(|line| defines_property(line))
			.filter_map(|line| line.split('.').next())
			.map(String::from)
			.collect()
	})
}

/// Does a line of a module's text start a property definition, `canvas.width := …` or `canvas.width(w) := …`
fn defines_property(line: &str) -> bool {
	let word = |text: &str| text.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(text.len());
	let object = word(line);
	object > 0 && line[object..].starts_with('.') && word(&line[object + 1..]) > 0 && line.contains(":=")
}

/// Why `object.property` is read-only: the comment above its getter, else above another getter of its object
fn read_only_reason(properties: &Properties, object: &str, property: &str) -> String {
	let comment_of = |name: &str| properties.get(&(object.to_string(), name.to_string()))?.comment.clone();
	let comment = comment_of(property).or_else(|| {
		properties.iter().filter(|((owner, _), _)| owner == object).find_map(|(_, known)| known.comment.clone())
	});
	let reason = comment.map_or(String::new(), |comment| format!(": {}", comment.trim()));
	format!("{object}.{property} is read-only, it has no setter{reason}")
}

pub fn lower(program: Node) -> Node {
	let names_standard_object = |name: &str| SIDES.contains(&name) || standard_objects().contains(name);
	let mut properties =
		if program.mentions_where(names_standard_object) { standard_properties() } else { Properties::new() };
	let mut own = Properties::new();
	let program = match with_instance_getters(program) {
		Ok(program) => program,
		Err(refused) => return refused,
	};
	let original = program.clone();
	collect(&crate::modules::statements(program.clone()), None, &mut own);
	let program = if own.is_empty() { program } else { without_definitions(program) };
	let bound = crate::program_names::Bindings::of(&program).bound();
	// a value (a variable, parameter or loop variable), not a function: `canvas() := Picture()` beside
	// `canvas.width := …` is fine
	// before tuples::lower the analyzer reads `[p, q] = …` as a definition of p: a destructured name is a value
	let functions = crate::analyzer::function_definitions(&original).user_functions;
	let destructured = destructured_variables(&original);
	let values: HashSet<String> = crate::program_names::Bindings::of(&original)
		.bound()
		.into_iter()
		.filter(|name| destructured.contains(name) || !functions.contains_key(name))
		.collect();
	if let Some(error) = refused_definition(&original, &values) {
		return error;
	}
	properties.extend(own);
	let mut rewrite = Rewrite { properties, bound, used: HashSet::new(), refused: None };
	let lowered = rewrite.rewritten(program);
	match rewrite.refused {
		Some(error) => error,
		None => crate::std_aliases::with_uses(lowered, rewrite.used),
	}
}

/// `Point.sum := it.x + it.y` of a class Point (card point-sum): a property of its instances, `it` the instance, which is
/// the function `sum(it: Point) := it.x + it.y`, read `Point(1, 2).sum` as any function of one instance is. Its setter
/// `Point.label(v) := it.name = v` (card point-label) is `label(it: Point, v) := { it.name = v; it }`, which
/// `p.label = "b"` calls as `p = label(p, "b")`, so it gives back the instance it changed. The instance may be written
/// `self` or `this` too, also as the explicit parameter: `Point.norm(self) := self.x * 2`, `Point.norm() := self.x`
/// (card point-norm, decision implicit-words-reserved). A property named as one of the class's fields would never be
/// read: a loud error
fn with_instance_getters(program: Node) -> Result<Node, Node> {
	let classes = crate::modules::needed_definitions::class_names(&program);
	if classes.is_empty() {
		return Ok(program);
	}
	let fields = crate::lowering::class_methods::class_fields(&program);
	let refused = std::cell::RefCell::new(None);
	let program = map_statements(program, &|statement| match instance_property(&statement, &classes) {
		Some(found)
			if fields.get(&found.class).is_some_and(|own| own.iter().any(|(field, _)| *field == found.property)) =>
		{
			let InstanceProperty { class, property, .. } = found;
			let reason = format!("{class}.{property} is a field of {class}: a property cannot share its name");
			refused.borrow_mut().get_or_insert_with(|| Diagnostic::at(&statement, reason).into_error());
			statement
		}
		Some(found) => found.definition(),
		None => statement,
	});
	refused.into_inner().map_or(Ok(program), Err)
}

/// The words a property of a class's instances may name the instance by
const RECEIVERS: [&str; 3] = [WRITTEN, crate::words::SELF_WORD, "this"];

/// `Point.sum := …` (a getter of a class's instances) or `Point.label(v) := …` (a setter, `value` its parameter)
struct InstanceProperty {
	class: String,
	property: String,
	receiver: String,
	value: Option<Node>,
	body: Node,
}

impl InstanceProperty {
	/// `sum(it: Point) := …`, `label(it: Point, v) := { …; it }`
	fn definition(self) -> Node {
		let receiver = || Node::Symbol(self.receiver.clone());
		let instance = Node::Key(Box::new(receiver()), Op::Colon, Box::new(Node::Symbol(self.class.clone())));
		let (parameters, body) = match self.value {
			None => (vec![instance], self.body),
			Some(value) => {
				let changed = Node::List(vec![self.body, receiver()], Bracket::Curly, Separator::Semicolon);
				(vec![instance, value], changed)
			}
		};
		key(crate::node::call(&self.property, parameters), Op::Define, body)
	}
}

fn instance_property(statement: &Node, classes: &HashSet<String>) -> Option<InstanceProperty> {
	let Node::Key(head, Op::Define, body) = statement.drop_meta() else {
		return None;
	};
	let Node::Key(object, Op::Dot, member) = head.drop_meta() else {
		return None;
	};
	let class = object.drop_meta().symbol_name().filter(|class| classes.contains(*class))?;
	let receiver_word =
		|node: &Node| node.drop_meta().symbol_name().filter(|word| RECEIVERS.contains(word)).map(String::from);
	let (property, explicit, value) = match member.drop_meta() {
		Node::List(items, Bracket::Round, _) => match items.as_slice() {
			[property] => (property, None, None),
			[property, receiver] if receiver_word(receiver).is_some() => (property, receiver_word(receiver), None),
			[property, value] => (property, None, Some(value.clone())),
			_ => return None,
		},
		_ => (member.as_ref(), None, None),
	};
	let mut read = None;
	body.visit(&mut |node| {
		read = read.take().or_else(|| receiver_word(node));
	});
	Some(InstanceProperty {
		class: class.to_string(),
		property: property.drop_meta().symbol_name()?.to_string(),
		receiver: explicit.or(read).unwrap_or(WRITTEN.to_string()),
		value,
		body: body.as_ref().clone(),
	})
}

/// The program with each top-level statement (also inside a block) mapped
fn map_statements(node: Node, map: &impl Fn(Node) -> Node) -> Node {
	match node {
		Node::List(items, bracket, separator) if bracket != Bracket::Curly => {
			Node::List(items.into_iter().map(|item| map_statements(item, map)).collect(), bracket, separator)
		}
		Node::Meta { node, data } => Node::Meta { node: Box::new(map_statements(*node, map)), data },
		statement => map(statement),
	}
}

/// A loud error for a property definition of a value the program binds, which no use would read: `p = {x: 1}; p.x := 5`
/// (the field is written `p.x = 5`)
fn refused_definition(program: &Node, values: &HashSet<String>) -> Option<Node> {
	// also inside a body: `f(p) := { p.x := 5; p }`
	let mut definitions = Vec::new();
	program.visit(&mut |node| {
		if definition(node).is_some() {
			definitions.push(node.clone());
		}
	});
	definitions.iter().find_map(|statement| {
		let (object, property, _, _) = definition(statement).filter(|(object, ..)| values.contains(object))?;
		let reason = format!("{object} is a value, so {object}.{property} := … defines nothing: write {object}.{property} = … to set its field");
		Some(Diagnostic::at(statement, reason).into_error())
	})
}

/// The program without its property definitions, which each use has taken in place
fn without_definitions(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) if bracket != Bracket::Curly => {
			let kept = items.into_iter().filter(|item| definition(item).is_none()).map(without_definitions);
			Node::List(kept.collect(), bracket, separator)
		}
		Node::Meta { node, data } => Node::Meta { node: Box::new(without_definitions(*node)), data },
		other => other,
	}
}

/// The variables the program's destructurings bind: `a, b = …`, `[a, b] = …`, `{x, y} = …`, `{x: a} = …`
fn destructured_variables(program: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	program.visit(&mut |node| names.extend(crate::tuples::destructured_names(node)));
	names
}

/// The fields `{width: 5}` or `class R{width: int}` declares
fn declared_fields(node: &Node) -> Vec<String> {
	let entries = match node.drop_meta() {
		Node::Type { body, .. } => body.drop_meta(),
		literal @ Node::List(_, Bracket::Curly, _) => literal,
		_ => return vec![],
	};
	let Node::List(entries, _, _) = entries else { return vec![] };
	let field = |entry: &Node| match entry.drop_meta() {
		Node::Key(name, Op::Colon, _) => name.drop_meta().symbol_name().map(String::from),
		_ => None,
	};
	entries.iter().filter_map(field).collect()
}

struct Rewrite {
	properties: Properties,
	bound: HashSet<String>,
	/// The modules the properties met need
	used: HashSet<&'static str>,
	/// The first write of a read-only property: the loud error
	refused: Option<Node>,
}

impl Rewrite {
	/// The property `object.property` names (`member` the property's word), unless the program binds the object
	/// (an other name of one, PROPERTY_ALIASES, names it with a hint)
	fn property_of(&self, object: &Node, member: &Node) -> Option<(String, String)> {
		let (object, property) = (object.drop_meta().symbol_name()?, member.drop_meta().symbol_name()?);
		if self.bound.contains(object) {
			return None;
		}
		let name = (object.to_string(), property.to_string());
		if self.properties.contains_key(&name) {
			return Some(name);
		}
		let (_, alias, canonical, note) =
			PROPERTY_ALIASES.iter().find(|(of, alias, ..)| *of == object && *alias == property)?;
		crate::normalize::hint(&format!("{object}.{alias}"), &format!("{object}.{canonical}"), note);
		Some((object.to_string(), canonical.to_string()))
	}

	fn dotted_property(&self, node: &Node) -> Option<(String, String)> {
		match node.drop_meta() {
			Node::Key(object, Op::Dot, member) => self.property_of(object, member),
			_ => None,
		}
	}

	/// The property's getter in place, its module brought along
	fn read(&mut self, name: &(String, String)) -> Option<Node> {
		let property = &self.properties[name];
		self.used.extend(property.module);
		property.getter.clone()
	}

	fn rewritten(&mut self, node: Node) -> Node {
		if let Some(name) = self.dotted_property(&node) {
			if let Some(getter) = self.read(&name) {
				return getter;
			}
		}
		if let Node::Key(target, op, value) = node.drop_meta() {
			let assigned = *op == Op::Assign || op.is_compound_assign();
			if let Some(name) = self.dotted_property(target).filter(|_| assigned) {
				return self.written(&name, *op, value.as_ref().clone(), &node);
			}
		}
		// `canvas.width(300)`: the setter called
		if let Node::Key(object, Op::Dot, member) = node.drop_meta() {
			if let Node::List(items, Bracket::Round, _) = member.drop_meta() {
				if let Some(name) = self.property_of(object, &items[0]).filter(|_| items.len() == 2) {
					return self.written(&name, Op::Assign, items[1].clone(), &node);
				}
			}
		}
		// `width(canvas)`, `width canvas`, `width(canvas, 300)`: the uniform calls of the getter and the setter
		// (decision setter-calls, notes/dispatch.md)
		if let Node::List(items, Bracket::Round, Separator::None) | Node::List(items, Bracket::None, Separator::Space) =
			node.drop_meta()
		{
			if let [word, object, value @ ..] = items.as_slice() {
				if let Some(name) = self.property_of(object, word).filter(|_| value.len() <= 1) {
					match value {
						[value] => return self.written(&name, Op::Assign, value.clone(), &node),
						_ => return self.read(&name).unwrap_or(node),
					}
				}
			}
		}
		if let Some(side) = self.bare_side(&node) {
			crate::normalize::set_position_of(&node);
			let reason = format!("a bare {side} is the canvas's; the others: window.{side}, display.{side}");
			crate::normalize::hint(side, &format!("{CANVAS}.{side}"), &reason);
			let canvas_side = (CANVAS.to_string(), side.to_string());
			return self.read(&canvas_side).expect("lib/draw.warp defines the canvas's sides");
		}
		match node {
			// a field's name is no read of the word: `{width: 5}`, `p.width`, `rect(width: 3)`
			Node::Key(name, Op::Colon, value) => Node::Key(name, Op::Colon, Box::new(self.rewritten(*value))),
			Node::Key(object, Op::Dot, member) => Node::Key(Box::new(self.rewritten(*object)), Op::Dot, member),
			other => {
				// a field binds its name inside the object or class declaring it, not elsewhere (`p = {width: 5}; width`)
				let fields: Vec<String> =
					declared_fields(&other).into_iter().filter(|field| self.bound.insert(field.clone())).collect();
				let rewritten = other.map_children(|child| self.rewritten(child));
				fields.iter().for_each(|field| {
					self.bound.remove(field);
				});
				rewritten
			}
		}
	}

	/// `object.property = value`, `+= value` and the like: the setter's body, its parameter the value written
	fn written(&mut self, name: &(String, String), op: Op, value: Node, statement: &Node) -> Node {
		let Some((parameter, setter)) = self.properties[name].setter.clone() else {
			let reason = read_only_reason(&self.properties, &name.0, &name.1);
			self.refused.get_or_insert_with(|| Diagnostic::at(statement, reason).into_error());
			return statement.clone();
		};
		let value = self.rewritten(value);
		let value = match (op, self.read(name)) {
			(Op::Assign, _) | (_, None) => value,
			(_, Some(current)) => key(current, op.base_op(), value),
		};
		self.used.extend(self.properties[name].module);
		crate::law::substitute(&setter, &HashMap::from([(parameter, value)]))
	}

	/// `width` or `height` alone, written in the program (not a module's file), which binds no such word
	fn bare_side(&self, node: &Node) -> Option<&'static str> {
		let name = node.drop_meta().symbol_name()?;
		let in_program = node.get_lineinfo().is_some_and(|at| at.file.is_none());
		let defined = |side: &str| self.properties.contains_key(&(CANVAS.to_string(), side.to_string()));
		SIDES.into_iter().find(|side| *side == name && !self.bound.contains(*side) && in_program && defined(side))
	}
}
