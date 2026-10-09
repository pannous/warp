//! Class fields typed with a unit (card unit-fields): `class Run{distance: km}` holds the SI amount like any quantity, the
//! class declares the field a number. An instance's unit fields have their signatures wherever static units knows the
//! instance's class: a constructor call, a variable declared `r: Run`, an element of a list (or table) declared `[Run]`,
//! a loop over such a list. A unit field read of an instance it cannot see is a loud error, never a silent SI number.

use super::{shown, Fields, Inference, Signature, Stop};
use crate::lowering::database_tables::OPTIONAL_MARK;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::HashMap;

/// What a unit field is declared to the emitter: its run-time amount is an exact or float SI amount
const NUMBER_TYPE: &str = "number";
const FOR_WORD: &str = "for";
const IN_WORD: &str = "in";

/// Each class with a field typed by a unit: its fields in order, each with its signature (none for a plain field)
pub(super) fn unit_classes(program: &Node) -> HashMap<String, Fields> {
	crate::class_methods::class_fields(program).into_iter().filter_map(|(class, fields)| {
		let fields: Fields = fields.into_iter().map(|(name, type_name)| {
			let signature = field_unit(&type_name).unwrap_or_default();
			(name, signature)
		}).collect();
		fields.iter().any(|(_, signature)| !signature.is_empty()).then_some((class, fields))
	}).collect()
}

/// `class Part{length: m ± 1 mm}`: a field type with a tolerance, which no class field holds yet (the tolerance goes in
/// the value, `Part(5 m ± 1 mm)`); loud, as dropping it would leave a plain `m` (card quantity-tolerance)
pub(super) fn field_tolerance(program: &Node) -> Option<String> {
	let mut found = None;
	program.visit(&mut |node| if let Node::Type { name, body } = node {
		body.visit(&mut |member| if let Node::Key(field, Op::Colon, type_name) = member {
			let is_unit_type = |unit: &Node| crate::units::unit_expression(unit).is_some();
			if found.is_none() && matches!(type_name.drop_meta(), Node::Key(unit, Op::PlusMinus, _) if is_unit_type(unit)) {
				found = Some(format!("{}.{}: {}", name.drop_meta().name(), field.drop_meta().name(), type_name.serialize().trim()));
			}
		});
	});
	found
}

/// The units the program's unit fields are written in: `distance: km` shows km like a written `5 km`
pub(super) fn written_field_units(program: &Node) -> Vec<crate::units::Factor> {
	let fields = crate::class_methods::class_fields(program).into_values().flatten();
	fields.filter_map(|(_, type_name)| field_units(&type_name)).flatten().collect()
}

/// The units of a field type that is a unit (`km`, `km/h`, an optional `km?`)
fn field_units(type_name: &str) -> Option<Vec<crate::units::Factor>> {
	let unit = type_name.trim_end_matches(OPTIONAL_MARK);
	if unit.is_empty() {
		return None;
	}
	crate::units::unit_expression(&crate::warp_parser::parse(unit))
}

/// The units written in the arguments of constructor calls of `classes` (`1500 m` of `Run(1500 m)`)
pub(super) fn stored_argument_units(program: &Node, classes: &HashMap<String, Fields>) -> Vec<&'static crate::units::Unit> {
	let mut stored = vec![];
	program.visit(&mut |node| {
		let Node::List(items, Bracket::Round, _) = node else { return };
		if let Some((_, arguments)) = items.split_first().filter(|(head, _)| classes.contains_key(&head.drop_meta().name())) {
			arguments.iter().for_each(|argument| stored.extend(super::written_units(argument)));
		}
	});
	stored
}

/// The signature of a field type that is a unit
fn field_unit(type_name: &str) -> Option<Signature> {
	field_units(type_name).map(|units| crate::units::signature(&units))
}

/// A field type that is a unit (`km`): the quantity it measures (`m` of km, `m/s` of km/h) and its SI amount per unit
/// (1000 of km); a stored column keeps it in its type (`NUMERIC km`, database.rs), the browser's store in its schema
pub(crate) fn unit_type(type_name: &str) -> Option<(String, f64)> {
	let units = field_units(type_name)?;
	let per_unit = crate::units::scale(&units, |factor| super::base_unit(factor.unit.dimension));
	Some((shown(&crate::units::signature(&units)), per_unit.to_f64()))
}

/// A constant quantity's SI amount and the quantity it measures, as unit_type gives a column's: `5 km` is (5000, "m")
pub(crate) fn si_quantity(node: &Node) -> Option<(f64, String)> {
	let Ok(crate::units::Value::Quantity(quantity)) = crate::units::evaluate(node) else { return None };
	let per_unit = crate::units::scale(&quantity.factors, |factor| super::base_unit(factor.unit.dimension));
	Some((quantity.amount.mul(&per_unit).to_f64(), shown(&crate::units::signature(&quantity.factors))))
}

/// The construction `P{…}` with `body` in place of its fields, its marks kept
/// A unit field given a run-time quantity `quantity(5 ± 1/100, "m")` (`Rope(5 m ± 1 cm)`, units.rs
/// lower_run_time_tolerances) holds it as written, `(5 ± 1/100) * m`: its amount in the field's base units (card
/// quantity-tolerance)
fn written_quantity(argument: &Node) -> Node {
	match crate::units::run_time_quantity_parts(argument) {
		Some((amount, unit)) => Node::Key(Box::new(amount.clone()), Op::Mul, Box::new(crate::warp_parser::parse(unit))),
		None => argument.clone(),
	}
}

fn with_instance_body(node: &Node, body: Node) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_instance_body(node, body)), data: data.clone() },
		Node::Key(name, op, _) => Node::Key(name.clone(), *op, Box::new(body)),
		other => other.clone(),
	}
}

/// `Run` of `r: Run`, and `Run` of a list `xs: [Run]` (true)
fn declared_class(declared: &Node) -> Option<(String, bool)> {
	match declared.drop_meta() {
		Node::Symbol(class) => Some((class.clone(), false)),
		Node::List(items, Bracket::Square, _) if items.len() == 1 => Some((items[0].drop_meta().name(), true)),
		_ => None,
	}
}

impl Inference {
	/// The class of an instance static units can see: a variable holding one, a constructor call, an element of a list or
	/// table of the class
	pub(super) fn instance_class(&self, node: &Node) -> Option<String> {
		if let Some(class) = self.named_construction_class(node) {
			return Some(class);
		}
		match node.drop_meta() {
			Node::Symbol(name) => self.instances.get(name).cloned(),
			Node::List(items, Bracket::Round, _) => match items.first().map(Node::drop_meta) {
				Some(Node::Symbol(class)) if self.classes.contains_key(class) => Some(class.clone()),
				_ => crate::database_tables::element_table(node).and_then(|table| self.class_lists.get(&table).cloned()),
			},
			Node::Key(list, Op::Hash, _) => self.class_lists.get(&list.drop_meta().name()).cloned(),
			_ => None,
		}
	}

	/// The class of a list literal of instances of one class: `[Run(1 km), Run(2 km)]`, or of another list of them (`rs =
	/// runs`, as a comprehension takes the list it filters; a table's list reads as `runs·load()`)
	fn instances_class(&self, list: &Node) -> Option<String> {
		let other_list = match list.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			_ => crate::database_tables::loaded_table(list),
		};
		if let Some(name) = other_list {
			return self.class_lists.get(&name).cloned();
		}
		let Node::List(items, Bracket::Square, _) = list.drop_meta() else { return None };
		let classes: Vec<String> = items.iter().map(|item| self.instance_class(item)).collect::<Option<_>>()?;
		classes.first().filter(|first| classes.iter().all(|class| class == *first)).cloned()
	}

	/// The fields of the instances a list of a class holds (a final `runs` shows their units)
	pub(super) fn list_instance_fields(&self, list: &Node) -> Option<Fields> {
		let name = crate::database_tables::loaded_table(list).unwrap_or_else(|| list.drop_meta().name());
		self.classes.get(self.class_lists.get(&name)?).cloned()
	}

	/// The fields of the instance `object` is, with their signatures
	pub(super) fn instance_fields(&self, object: &Node) -> Option<Fields> {
		self.classes.get(&self.instance_class(object)?).cloned()
	}

	/// `r` holds an instance of `class` from now on
	pub(super) fn declare_instance(&mut self, name: String, class: String) {
		if let Some(fields) = self.classes.get(&class) {
			self.objects.insert(name.clone(), fields.clone());
			self.instances.insert(name, class);
		}
	}

	/// `r = Run(5 km)`, `r = runs#1`: the assignment of an instance, its variable declared of its class; `rs = [Run(1 km)]`
	/// of a list of instances of one class
	pub(super) fn assigned_instance(&mut self, name: &str, target: &Node, op: Op, value: &Node) -> Option<Result<(Node, Signature), Stop>> {
		if let Some(class) = self.instances_class(value) {
			return Some(self.infer(value.clone()).map(|(value, _)| {
				self.class_lists.insert(name.to_string(), class);
				(Node::Key(Box::new(target.clone()), op, Box::new(value)), vec![])
			}));
		}
		let class = self.instance_class(value)?;
		Some(self.infer(value.clone()).map(|(value, _)| {
			self.declare_instance(name.to_string(), class);
			(Node::Key(Box::new(target.clone()), op, Box::new(value)), vec![])
		}))
	}

	/// `r: Run = …` declares r an instance, `runs: [Run] = …` a list of them
	pub(super) fn declared_instance(&mut self, name: &str, declared: &Node) {
		let Some((class, is_list)) = declared_class(declared).filter(|(class, _)| self.classes.contains_key(class)) else { return };
		match is_list {
			true => { self.class_lists.insert(name.to_string(), class); }
			false => self.declare_instance(name.to_string(), class),
		}
	}

	/// `for r in runs {…}` over a list of a class: r is an instance of it in the body
	pub(super) fn loop_instance(&mut self, items: &[Node]) {
		let [word, variable, in_word, iterable, _] = items else { return };
		let is_word = |node: &Node, word: &str| matches!(node.drop_meta(), Node::Symbol(name) if name == word);
		if !is_word(word, FOR_WORD) || !is_word(in_word, IN_WORD) {
			return;
		}
		if let (Node::Symbol(name), Some(class)) = (variable.drop_meta(), self.instances_class(iterable)) {
			self.declare_instance(name.clone(), class);
		}
	}

	/// `Run(5 km, "park")`: each argument has its field's signature (a plain number for a unit field is a DimensionError);
	/// ø has no dimension, whether a field may hold nothing is its type's matter (`distance: km?`)
	pub(super) fn constructed(&mut self, items: &[Node]) -> Option<Result<(Node, Signature), Stop>> {
		let (head, arguments) = items.split_first()?;
		let fields = self.classes.get(&head.drop_meta().name())?.clone();
		let class = head.drop_meta().name();
		let mut lowered = vec![head.clone()];
		for (index, argument) in arguments.iter().enumerate() {
			if super::is_nothing(argument) {
				lowered.push(argument.clone());
				continue;
			}
			let (argument, given) = match self.infer(written_quantity(argument)) {
				Ok(inferred) => inferred,
				Err(stop) => return Some(Err(stop)),
			};
			if let Some((field, held)) = fields.get(index).filter(|(_, held)| *held != given) {
				return Some(Err(Stop::Error(format!("DimensionError: {class}.{field} holds {}, is given {}", shown(held), shown(&given)))));
			}
			lowered.push(argument);
		}
		Some(Ok((Node::List(lowered, Bracket::Round, crate::node::Separator::None), vec![])))
	}

	/// `Run` and the fields of a construction with named fields `Run{distance: 5 km}` of a class with unit fields; the
	/// construction may still be plain data, as type_constructor leaves a class with unit field types unmarked
	fn named_construction_parts<'n>(&self, node: &'n Node) -> Option<(String, &'n Node)> {
		let (name, body) = match crate::type_constructor::instance_parts(node) {
			Some(parts) => parts,
			None => match node.drop_meta() {
				Node::Key(name, Op::Colon | Op::None, body) => (name.as_ref(), body.as_ref()),
				_ => return None,
			},
		};
		let class = name.drop_meta().name();
		let is_named_fields = matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _));
		(is_named_fields && matches!(name.drop_meta(), Node::Symbol(_)) && self.classes.contains_key(&class)).then_some((class, body))
	}

	fn named_construction_class(&self, node: &Node) -> Option<String> {
		self.named_construction_parts(node).map(|(class, _)| class)
	}

	/// `Run{distance: 5 km}`: each named field has its field's signature, as the arguments of `Run(5 km)` (card map-units)
	pub(super) fn named_construction(&mut self, node: &Node) -> Option<Result<(Node, Signature), Stop>> {
		let (class, body) = self.named_construction_parts(node)?;
		let fields = self.classes.get(&class)?.clone();
		let Node::List(entries, bracket, separator) = body.drop_meta() else { return None };
		let mut lowered = vec![];
		for entry in entries {
			let Node::Key(field, op, value) = entry.drop_meta() else {
				lowered.push(entry.clone());
				continue;
			};
			let (value, given) = match self.infer(written_quantity(value)) {
				Ok(inferred) => inferred,
				Err(stop) => return Some(Err(stop)),
			};
			let held = fields.iter().find(|(name, _)| *name == field.drop_meta().name()).map(|(_, held)| held);
			if let Some(held) = held.filter(|held| **held != given && !super::is_nothing(&value)) {
				return Some(Err(Stop::Error(format!("DimensionError: {class}.{} holds {}, is given {}", field.drop_meta().name(), shown(held), shown(&given)))));
			}
			lowered.push(Node::Key(field.clone(), *op, Box::new(value)));
		}
		Some(Ok((with_instance_body(node, Node::List(lowered, bracket.clone(), separator.clone())), vec![])))
	}

	/// The class declaration with its unit fields declared numbers; a default value (`distance: km = 0 km`) has the unit
	pub(super) fn class_declaration(&mut self, name: Box<Node>, body: Box<Node>) -> Result<(Node, Signature), Stop> {
		let Some(fields) = self.classes.get(&name.drop_meta().name()).cloned() else {
			return Ok((Node::Type { name, body }, vec![]));
		};
		let mut failure = None;
		let body = body.map_children(|item| match self.numeric_field(item.clone(), &fields) {
			Ok(field) => field,
			Err(stop) => {
				failure.get_or_insert(stop);
				item
			}
		});
		match failure {
			Some(stop) => Err(stop),
			None => Ok((Node::Type { name, body: Box::new(body) }, vec![])),
		}
	}

	fn numeric_field(&mut self, item: Node, fields: &Fields) -> Result<Node, Stop> {
		let unit_of = |field: &Node| fields.iter().find(|(name, signature)| *name == field.drop_meta().name() && !signature.is_empty()).map(|(_, signature)| signature.clone());
		match item {
			Node::Meta { node, data } => Ok(Node::Meta { node: Box::new(self.numeric_field(*node, fields)?), data }),
			Node::Key(field, Op::Colon, field_type) if unit_of(&field).is_some() => {
				let number_type = |written: &Node| {
					let optional = if written.drop_meta().name().ends_with(OPTIONAL_MARK) { OPTIONAL_MARK.to_string() } else { String::new() };
					Box::new(Node::Symbol(format!("{NUMBER_TYPE}{optional}")))
				};
				let number = match field_type.drop_meta() {
					Node::Type { name, body } => Node::Type { name: number_type(name), body: body.clone() },
					other => *number_type(other),
				};
				Ok(Node::Key(field, Op::Colon, Box::new(number)))
			}
			Node::Key(declaration, Op::Assign, default) => {
				let held = match declaration.drop_meta() {
					Node::Key(field, Op::Colon, _) => unit_of(field),
					_ => None,
				};
				let Some(held) = held else { return Ok(Node::Key(declaration, Op::Assign, default)) };
				let (default, given) = self.infer(*default)?;
				if given != held {
					return Err(Stop::Error(format!("DimensionError: the default of {} is {}, the field holds {}", declaration.serialize().trim(), shown(&given), shown(&held))));
				}
				Ok(Node::Key(Box::new(self.numeric_field(*declaration, fields)?), Op::Assign, Box::new(default)))
			}
			other => Ok(other),
		}
	}

	/// `x.distance` of an instance static units cannot see, where distance is some class's unit field: loud
	pub(super) fn unseen_unit_field(&self, object: &Node, field: &Node) -> Option<Stop> {
		let field = field.drop_meta().name();
		let class = self.classes.iter().find(|(_, fields)| fields.iter().any(|(name, signature)| *name == field && !signature.is_empty()))?.0;
		Some(Stop::Error(format!("{}.{field}: the unit of {class}.{field} is known of an instance declared so: `x: {class} = …`, a constructor call, an element of a list declared [{class}]", object.serialize().trim())))
	}
}
