//! Properties of the standard objects (card make-width, decision canvas-size): `window.width`, `display.height`,
//! `canvas.width` read like fields and, where the platform can, are written like them: `window.width = 800` resizes
//! the paint window (the playground: the paintings' frame), `canvas.width = 300` the canvas of lib/draw.warp (a new,
//! blank one, as an HTML canvas does), while `display.width = 5` is a loud error. Each property is a getter and a setter
//! written in warp, `it` the value written; `+=` and its kin write what they read. A bare `width` or `height` the
//! program does not bind is the canvas's, with a did-you-mean naming the three. A program binding the object or the
//! word itself (`canvas = {width: 9}`, `f(width) := …`) keeps its own; a field name (`{width: 5}`, `p.width`) is none.
//! Runs before modules::resolve, which then brings lib/draw.warp, and before system_values marks the window's reads.

use super::nodes::{filled, for_in_variable, function_head, key, parameter_name};
use crate::errors::diagnostic::Diagnostic;
use crate::node::Node;
use crate::operators::Op;
use std::collections::HashSet;

/// The placeholder of the written value in a setter
const WRITTEN: &str = "it";
const DRAW_MODULE: &str = "draw";
const CANVAS: &str = "canvas";
const SIDES: [&str; 2] = ["width", "height"];
const DISPLAY_READ_ONLY: &str = "a program cannot resize the display: set window.width or canvas.width instead";

/// `object.property`: read as `getter`, written as `setter` (None: read only), needing `module`
struct Property {
	object: &'static str,
	property: &'static str,
	getter: &'static str,
	setter: Option<&'static str>,
	module: Option<&'static str>,
}

const fn canvas_side(property: &'static str, getter: &'static str, setter: &'static str) -> Property {
	Property { object: CANVAS, property, getter, setter: Some(setter), module: Some(DRAW_MODULE) }
}

const fn system_side(
	object: &'static str,
	property: &'static str,
	getter: &'static str,
	setter: Option<&'static str>,
) -> Property {
	Property { object, property, getter, setter, module: None }
}

const PROPERTIES: [Property; 7] = [
	canvas_side("width", "canvas_width", "canvas(it, canvas_height)"),
	canvas_side("height", "canvas_height", "canvas(canvas_width, it)"),
	// system values (warp-runtime system_values.rs), so `whenever window.width > 800 {…}` listens
	system_side("window", "width", "window_width", Some("set_system_value(\"window_width\", it)")),
	system_side("window", "height", "window_height", Some("set_system_value(\"window_height\", it)")),
	system_side("display", "width", "display_width", None),
	system_side("display", "height", "display_height", None),
	// the device pixels per inch, a quantity in dpi that turns lengths into px, `3 cm * display.dpi`; a program sets it
	// in dpi, `display.dpi = 300 dpi`
	system_side("display", "dpi", "display_dpi * 1dpi", Some("set_system_value(\"display_dpi\", it / 1dpi)")),
];

pub fn lower(program: Node) -> Node {
	let mut rewrite = Rewrite { bound: bound_names(&program), used: HashSet::new(), refused: None };
	let lowered = rewrite.rewritten(program);
	match rewrite.refused {
		Some(error) => error,
		None => crate::std_aliases::with_uses(lowered, rewrite.used),
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
	bound: HashSet<String>,
	/// The modules the properties met need
	used: HashSet<&'static str>,
	/// The first write of a read-only property: the loud error
	refused: Option<Node>,
}

impl Rewrite {
	/// The property `object.property` names, unless the program binds the object
	fn property_of(&self, node: &Node) -> Option<&'static Property> {
		let Node::Key(object, Op::Dot, property) = node.drop_meta() else {
			return None;
		};
		let (object, property) = (object.drop_meta().symbol_name()?, property.drop_meta().symbol_name()?);
		PROPERTIES
			.iter()
			.find(|known| known.object == object && known.property == property && !self.bound.contains(object))
	}

	/// The property's getter, its module brought along
	fn read(&mut self, property: &Property) -> Node {
		self.used.extend(property.module);
		filled(property.getter, [])
	}

	fn rewritten(&mut self, node: Node) -> Node {
		if let Some(property) = self.property_of(&node) {
			return self.read(property);
		}
		if let Node::Key(target, op, value) = node.drop_meta() {
			if let Some(property) = self.property_of(target).filter(|_| *op == Op::Assign || op.is_compound_assign()) {
				return self.written(property, *op, value.as_ref().clone(), &node);
			}
		}
		if let Some(side) = self.bare_side(&node) {
			crate::normalize::set_position_of(&node);
			let reason = format!("a bare {side} is the canvas's; the others: window.{side}, display.{side}");
			crate::normalize::hint(side, &format!("{CANVAS}.{side}"), &reason);
			let canvas_side = PROPERTIES.iter().find(|known| known.object == CANVAS && known.property == side);
			return self.read(canvas_side.expect("the canvas's sides"));
		}
		match node {
			// a field's name is no read of the word: `{width: 5}`, `p.width`, `rect(width: 3)`
			Node::Key(name, Op::Colon, value) => Node::Key(name, Op::Colon, Box::new(self.rewritten(*value))),
			Node::Key(object, Op::Dot, member) => Node::Key(Box::new(self.rewritten(*object)), Op::Dot, member),
			other => other.map_children(|child| self.rewritten(child)),
		}
	}

	/// `object.property = value`, `+= value` and the like: the setter of what is written
	fn written(&mut self, property: &'static Property, op: Op, value: Node, statement: &Node) -> Node {
		let Some(setter) = property.setter else {
			self.refused.get_or_insert_with(|| Diagnostic::at(statement, DISPLAY_READ_ONLY.to_string()).into_error());
			return statement.clone();
		};
		let value = self.rewritten(value);
		let value = if op == Op::Assign { value } else { key(self.read(property), op.base_op(), value) };
		self.used.extend(property.module);
		filled(setter, [(WRITTEN, value)])
	}

	/// `width` or `height` alone, written in the program (not a module's file), which binds no such word
	fn bare_side(&self, node: &Node) -> Option<&'static str> {
		let name = node.drop_meta().symbol_name()?;
		let in_program = node.get_lineinfo().is_some_and(|at| at.file.is_none());
		SIDES.into_iter().find(|side| *side == name && !self.bound.contains(*side) && in_program)
	}
}
