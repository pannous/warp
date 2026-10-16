//! Properties of the standard objects (card make-width, decision canvas-size): `window.width`, `display.height`,
//! `canvas.width` read like fields and, where the platform can, are written like them: `window.width = 800` resizes
//! the paint window (the playground: the paintings' frame), `canvas.width = 300` the canvas of lib/draw.warp (a new,
//! blank one, as an HTML canvas does), while `display.width = 5` is a loud error. Each property is a getter and a setter
//! written in warp, `it` the value written; `+=` and its kin write what they read. A bare `width` or `height` the
//! program does not bind is the canvas's, with a did-you-mean naming the three. A program binding the object or the
//! word itself (`canvas = {width: 9}`, `f(width) := …`) keeps its own; a field name (`{width: 5}`, `p.width`) is none.
//! Runs before modules::resolve, which then brings lib/draw.warp, and before system_values marks the window's reads.

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

use super::nodes::{for_in_variable, function_head, key, parameter_name};
use crate::errors::diagnostic::Diagnostic;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

/// The parameter of a setter written getter-shaped: `canvas.width := canvas(it, canvas_height)`
const WRITTEN: &str = "it";
const CANVAS: &str = "canvas";
const SIDES: [&str; 2] = ["width", "height"];

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
				let parsed = crate::normalize::without_hints(|| crate::warp_parser::WarpParser::parse(source));
				let needed = (module != crate::modules::PRELUDE_MODULE).then_some(module);
				collect(&crate::modules::statements(parsed), needed, &mut properties);
			}
			properties
		};
	}
	PROPERTIES.with(Properties::clone)
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
	let mut properties = standard_properties();
	let mut own = Properties::new();
	collect(&crate::modules::statements(program.clone()), None, &mut own);
	let program = if own.is_empty() { program } else { without_definitions(program) };
	properties.extend(own);
	let mut rewrite = Rewrite { properties, bound: bound_names(&program), used: HashSet::new(), refused: None };
	let lowered = rewrite.rewritten(program);
	match rewrite.refused {
		Some(error) => error,
		None => crate::std_aliases::with_uses(lowered, rewrite.used),
	}
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

/// The names the program gives: variables, functions, their parameters (`def f(width) {…}` too), loop variables,
/// fields; a property written (`canvas.width = 3`) binds no name
pub(crate) fn bound_names(program: &Node) -> HashSet<String> {
	let mut bound = crate::soft_keywords::names_given(program);
	program.visit(&mut |node| {
		if let Some(head) = function_head(node) {
			bound.extend(head.parameters().into_iter().filter_map(parameter_name));
		}
		match node {
			Node::Key(target, op, _) if op.is_write() || *op == Op::Define => {
				bound.extend(target_name(target).map(String::from))
			}
			Node::List(items, _, _) => bound.extend(for_in_variable(items)),
			_ => {}
		}
	});
	bound
}

/// `height` of a write's target `height` or a declared `height: length`
fn target_name(target: &Node) -> Option<&str> {
	match target.drop_meta() {
		Node::Key(name, Op::Colon, _) => name.drop_meta().symbol_name(),
		other => other.symbol_name(),
	}
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
	fn property_of(&self, object: &Node, member: &Node) -> Option<(String, String)> {
		let (object, property) = (object.drop_meta().symbol_name()?, member.drop_meta().symbol_name()?);
		let name = (object.to_string(), property.to_string());
		(self.properties.contains_key(&name) && !self.bound.contains(object)).then_some(name)
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
			other => other.map_children(|child| self.rewritten(child)),
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
