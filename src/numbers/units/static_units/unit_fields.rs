//! Class fields typed with a unit (card unit-fields): `class Run{distance: km}` holds the SI amount like any quantity, the
//! class declares the field a number. An instance's unit fields have their signatures wherever static units knows the
//! instance's class: a constructor call, a variable declared `r: Run`, an element of a list (or table) declared `[Run]`,
//! a loop over such a list. A unit field read of an instance it cannot see is a loud error, never a silent SI number.

use super::list_units::CALLS_FOLLOWED;
use super::{shown, Fields, Inference, Signature, Stop};
use crate::lowering::database_tables::OPTIONAL_MARK;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::words::{FOR_WORD, IN_WORD, NUMBER_WORD};
use std::collections::HashMap;

/// Each class with a field typed by a unit: its fields in order, each with its signature (none for a plain field)
pub(super) fn unit_classes(program: &Node) -> HashMap<String, Fields> {
	crate::class_methods::class_fields(program)
		.into_iter()
		.filter_map(|(class, fields)| {
			let fields: Fields = fields
				.into_iter()
				.map(|(name, type_name)| {
					let signature = field_unit(&type_name).unwrap_or_default();
					(name, signature)
				})
				.collect();
			fields.iter().any(|(_, signature)| !signature.is_empty()).then_some((class, fields))
		})
		.collect()
}

/// Each field typed with a tolerance (`length: m ± 1 mm`) by class: its position among the fields, its name, its spread
type Tolerances = HashMap<String, Vec<(usize, String, Node)>>;

/// `class Part{length: m ± 1 mm}`: the value carries the tolerance (user decision P233): `Part(5 m)` holds `5 m ± 1 mm`
/// (a run-time quantity, lower_run_time_tolerances runs next), a value given with its own tolerance keeps it, and the
/// field type is the plain unit
pub(crate) fn lower_field_tolerances(program: Node) -> Node {
	let tolerances = field_tolerances(&program);
	if tolerances.is_empty() {
		return program;
	}
	with_field_tolerances(program, &tolerances)
}

fn field_tolerances(program: &Node) -> Tolerances {
	let fields = crate::class_methods::class_fields(program);
	let mut tolerances = Tolerances::new();
	program.visit(&mut |node| {
		if let Node::Type { name, body } = node {
			let class = name.drop_meta().name();
			body.visit(&mut |member| {
				if let Some((field, _, spread)) = tolerance_parts(member) {
					let position =
						fields.get(&class).and_then(|fields| fields.iter().position(|(name, _)| *name == field));
					if let Some(position) = position {
						tolerances.entry(class.clone()).or_default().push((position, field, spread.clone()));
					}
				}
			});
		}
	});
	tolerances
}

/// `length: m ± 1 mm`: the field name, its unit and its spread
fn tolerance_parts(member: &Node) -> Option<(String, &Node, &Node)> {
	let Node::Key(field, Op::Colon, type_name) = member.drop_meta() else {
		return None;
	};
	match type_name.drop_meta() {
		Node::Key(unit, Op::PlusMinus, spread) if crate::numbers::units::unit_expression(unit).is_some() => {
			Some((field.drop_meta().name(), unit, spread))
		}
		_ => None,
	}
}

fn with_field_tolerances(node: Node, tolerances: &Tolerances) -> Node {
	let toleranced = |class: &str, field: &str, value: Node| {
		let spread = tolerances.get(class).and_then(|fields| fields.iter().find(|(_, name, _)| name == field));
		match (spread, value.drop_meta()) {
			(None, _) | (_, Node::Key(_, Op::PlusMinus, _)) => value,
			(Some((_, _, spread)), _) => Node::Key(Box::new(value), Op::PlusMinus, Box::new(spread.clone())),
		}
	};
	let fields = crate::class_methods::class_fields(&node);
	let field_names = |class: &str| {
		tolerances
			.contains_key(class)
			.then(|| fields.get(class).map(|fields| fields.iter().map(|(name, _)| name.clone()).collect()))
			.flatten()
	};
	let node = with_field_values(node, &field_names, &toleranced);
	without_field_spreads(node)
}

/// `class Display{density: dpi}`: each unit field's unit word as written, kept for the program's messages and fixes
/// (`Display.density holds dpi`), before lower_unit_words expands `dpi` to `px/inch`
pub(crate) fn remember_field_unit_words(program: Node) -> Node {
	let words = crate::class_methods::class_fields(&program)
		.into_iter()
		.flat_map(|(class, fields)| fields.into_iter().map(move |(field, type_name)| (class.clone(), field, type_name)))
		.filter(|(_, _, type_name)| is_unit_type(type_name))
		.map(|(class, field, type_name)| {
			(format!("{class}.{field}"), type_name.trim_end_matches(OPTIONAL_MARK).to_string())
		});
	let words = words.collect();
	crate::compiler::compile_context::change(|context| context.field_unit_words = words);
	program
}

/// The unit word a field `Display.density` was written with, `dpi`
pub(super) fn field_unit_word(field: &str) -> Option<String> {
	crate::compiler::compile_context::read(|context| context.field_unit_words.get(field).cloned())
}

/// `96` given to a field of `unit` (`dpi`): its amount in SI units, `96 · 5000/127`, with the warning offering `96 dpi`
fn in_unit(field: &str, unit: &str, written: &Node, value: Node) -> Option<Node> {
	let aliases = crate::numbers::units::unit_aliases();
	let expression = aliases.iter().find(|(alias, _)| *alias == unit).map_or(unit, |(_, expression)| expression);
	let one_unit = crate::numbers::units::unit_expression(&crate::warp_parser::parse(expression))?;
	let per_unit = crate::numbers::units::scale(&one_unit, |factor| super::base_unit(factor.unit.dimension));
	let text = written.serialize().trim().to_string();
	let with_unit = super::with_unit_written(written, unit);
	let message = format!("{field} holds {unit}: write the unit, `{with_unit}`");
	let warning = crate::errors::diagnostic::Diagnostic::at(written, message);
	crate::errors::diagnostic::say(warning.offer("the field's unit", text, with_unit));
	Some(Node::Key(Box::new(value), Op::Mul, Box::new(super::number_node(&per_unit)?)))
}

/// `km`, `km/h`, and before lower_unit_words a unit word `dpi` (a word for px/inch)
fn is_unit_type(type_name: &str) -> bool {
	let unit = type_name.trim_end_matches(OPTIONAL_MARK);
	field_units(unit).is_some() || crate::numbers::units::names_unit(unit)
}

/// `class Part{length: m ± 1 mm}` declares the plain unit `length: m`
fn without_field_spreads(node: Node) -> Node {
	match node {
		Node::Type { name, body } => Node::Type {
			name,
			body: Box::new(body.map_children(|member| match tolerance_parts(&member) {
				Some((field, unit, _)) => Node::Key(Box::new(Node::Symbol(field)), Op::Colon, Box::new(unit.clone())),
				None => member,
			})),
		},
		other => other.map_children(without_field_spreads),
	}
}

/// Each value given to a field of a class `fields_of` names the fields of, in a construction `Run(5 km)`, `Run(distance:
/// 5 km)` or `Run{distance: 5 km}`, as `value_for(class, field, value)` makes it
fn with_field_values(
	node: Node,
	fields_of: &dyn Fn(&str) -> Option<Vec<String>>,
	value_for: &dyn Fn(&str, &str, Node) -> Node,
) -> Node {
	let walk = |child: Node| with_field_values(child, fields_of, value_for);
	let class_fields = |class: &Node| Some((class.drop_meta().name(), fields_of(&class.drop_meta().name())?));
	let named = |class: &str, fields: &[String], entry: Node| match entry.drop_meta() {
		Node::Key(field, op @ (Op::Colon | Op::None), value) if fields.contains(&field.drop_meta().name()) => {
			let value = value_for(class, &field.drop_meta().name(), walk(*value.clone()));
			Some(Node::Key(field.clone(), *op, Box::new(value)))
		}
		_ => None,
	};
	match node {
		Node::List(items, Bracket::Round, separator) if items.first().and_then(class_fields).is_some() => {
			let (class, fields) = items.first().and_then(class_fields).unwrap();
			let items = items.into_iter().enumerate().map(|(index, item)| match index {
				0 => item,
				_ => named(&class, &fields, item.clone()).unwrap_or_else(|| match fields.get(index - 1) {
					Some(field) if !item.is_empty_node() => value_for(&class, field, walk(item)),
					_ => walk(item),
				}),
			});
			Node::List(items.collect(), Bracket::Round, separator)
		}
		Node::Key(class, op @ (Op::Colon | Op::None), body)
			if class_fields(&class).is_some() && matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) =>
		{
			let (name, fields) = class_fields(&class).unwrap();
			let body = body.map_children(|entry| named(&name, &fields, entry.clone()).unwrap_or_else(|| walk(entry)));
			Node::Key(class, op, Box::new(body))
		}
		other => other.map_children(walk),
	}
}

/// The units the program's unit fields are written in: `distance: km` shows km like a written `5 km`
pub(super) fn written_field_units(program: &Node) -> Vec<crate::numbers::units::Factor> {
	let fields = crate::class_methods::class_fields(program).into_values().flatten();
	fields.filter_map(|(_, type_name)| field_units(&type_name)).flatten().collect()
}

/// The units of a field type that is a unit (`km`, `km/h`, an optional `km?`)
fn field_units(type_name: &str) -> Option<Vec<crate::numbers::units::Factor>> {
	let unit = type_name.trim_end_matches(OPTIONAL_MARK);
	if unit.is_empty() {
		return None;
	}
	crate::numbers::units::unit_expression(&crate::warp_parser::parse(unit))
}

/// The units written in the arguments of constructor calls of `classes` (`1500 m` of `Run(1500 m)`)
pub(super) fn stored_argument_units(
	program: &Node,
	classes: &HashMap<String, Fields>,
) -> Vec<&'static crate::numbers::units::Unit> {
	let mut stored = vec![];
	program.visit(&mut |node| {
		let Node::List(items, Bracket::Round, _) = node else {
			return;
		};
		if let Some((_, arguments)) =
			items.split_first().filter(|(head, _)| classes.contains_key(&head.drop_meta().name()))
		{
			arguments.iter().for_each(|argument| stored.extend(super::written_units(argument)));
		}
	});
	stored
}

/// The signature of a field type that is a unit
fn field_unit(type_name: &str) -> Option<Signature> {
	field_units(type_name).map(|units| super::signature(&units))
}

/// A field type that is a unit (`km`): the quantity it measures (`m` of km, `m/s` of km/h) and its SI amount per unit
/// (1000 of km); a stored column keeps it in its type (`NUMERIC km`, database.rs), the browser's store in its schema
pub(crate) fn unit_type(type_name: &str) -> Option<(String, f64)> {
	let units = field_units(type_name)?;
	let per_unit = crate::numbers::units::scale(&units, |factor| super::base_unit(factor.unit.dimension));
	Some((shown(&super::signature(&units)), per_unit.to_f64()))
}

/// A constant quantity's SI amount and the quantity it measures, as unit_type gives a column's: `5 km` is (5000, "m")
pub(crate) fn si_quantity(node: &Node) -> Option<(f64, String)> {
	let (amount, signature) = super::constant(node).ok()??;
	(!signature.is_empty()).then(|| Some((super::exact_amount(&amount)?.to_f64(), shown(&signature.into()))))?
}

/// The construction `P{…}` with `body` in place of its fields, its marks kept
/// A unit field given a run-time quantity `quantity(5 ± 1/100, "m")` (`Rope(5 m ± 1 cm)`, units.rs
/// lower_run_time_tolerances) holds it as written, `(5 ± 1/100) * m`: its amount in the field's base units (card
/// quantity-tolerance)
fn written_quantity(argument: &Node) -> Node {
	match crate::numbers::units::run_time_quantity_parts(argument) {
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
	/// table of the class, a call of a function whose last value is one of these
	pub(super) fn instance_class(&self, node: &Node) -> Option<String> {
		self.instance_class_within(node, CALLS_FOLLOWED)
	}

	/// `f(96)` of `f(n) := D(n * 1 inch)` is a D: the calls followed into function bodies are at most `calls_left` deep
	fn instance_class_within(&self, node: &Node, calls_left: usize) -> Option<String> {
		if let Some(class) = self.named_construction_class(node) {
			return Some(class);
		}
		match node.drop_meta() {
			Node::Symbol(name) => self.instances.get(name).cloned(),
			Node::List(items, Bracket::Round, _) => match items.first().map(Node::drop_meta) {
				Some(Node::Symbol(class)) if self.classes.contains_key(class) => Some(class.clone()),
				_ => match self.returned_by_call(items, calls_left) {
					Some(last) => self.instance_class_within(last, calls_left - 1),
					None => crate::database_tables::element_table(node)
						.and_then(|table| self.class_lists.get(&table).cloned()),
				},
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
		let Node::List(items, Bracket::Square, _) = list.drop_meta() else {
			return None;
		};
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
	pub(super) fn assigned_instance(
		&mut self,
		name: &str,
		target: &Node,
		op: Op,
		value: &Node,
	) -> Option<Result<(Node, Signature), Stop>> {
		if let Some(class) = self.instances_class(value) {
			return Some(self.infer(value.clone()).map(|(value, _)| {
				self.class_lists.insert(name.to_string(), class);
				(Node::Key(Box::new(target.clone()), op, Box::new(value)), Signature::plain())
			}));
		}
		let class = self.instance_class(value)?;
		Some(self.infer(value.clone()).map(|(value, _)| {
			self.declare_instance(name.to_string(), class);
			(Node::Key(Box::new(target.clone()), op, Box::new(value)), Signature::plain())
		}))
	}

	/// `r: Run = …` declares r an instance, `runs: [Run] = …` a list of them
	pub(super) fn declared_instance(&mut self, name: &str, declared: &Node) {
		let Some((class, is_list)) = declared_class(declared).filter(|(class, _)| self.classes.contains_key(class))
		else {
			return;
		};
		match is_list {
			true => {
				self.class_lists.insert(name.to_string(), class);
			}
			false => self.declare_instance(name.to_string(), class),
		}
	}

	/// `for r in runs {…}` over a list of a class: r is an instance of it in the body
	pub(super) fn loop_instance(&mut self, items: &[Node]) {
		let [word, variable, in_word, iterable, _] = items else {
			return;
		};
		let is_word = |node: &Node, word: &str| matches!(node.drop_meta(), Node::Symbol(name) if name == word);
		if !is_word(word, FOR_WORD) || !is_word(in_word, IN_WORD) {
			return;
		}
		if let (Node::Symbol(name), Some(class)) = (variable.drop_meta(), self.instances_class(iterable)) {
			self.declare_instance(name.clone(), class);
		}
	}

	/// The value written for a unit field (`Display.density`, shown as `field`): a quantity of its kind, or a plain
	/// number taken in the field's written unit with a warning whose fix writes it (decision unit-field-plain-number);
	/// anything else is a DimensionError naming the unit as written
	pub(super) fn field_value(
		&mut self,
		field: &str,
		key: &str,
		held: Option<&Signature>,
		written: &Node,
	) -> Result<Node, Stop> {
		let (value, given) = self.infer(written_quantity(written))?;
		let Some(held) = held.filter(|held| **held != given && !value.is_empty_node()) else {
			return Ok(value);
		};
		let unit = field_unit_word(key).unwrap_or_else(|| shown(held));
		// `holds m/s (mph)`: the kind of quantity, and the unit as written where it differs
		let holds = match unit == shown(held) {
			true => unit.clone(),
			false => format!("{} ({unit})", shown(held)),
		};
		match given.is_empty() && !given.is_list() {
			true => in_unit(field, &unit, written, value).ok_or(Stop::Unsupported),
			false => Err(Stop::Error(format!("DimensionError: {field} holds {holds}, is given {}", shown(&given)))),
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
			if argument.is_empty_node() {
				lowered.push(argument.clone());
				continue;
			}
			if let Some(stop) = self.unit_after_variable(argument, arguments.get(index + 1)) {
				return Some(Err(stop));
			}
			// `Run(distance: 5 km)`: a named argument has its field's signature
			let named = match argument.drop_meta() {
				Node::Key(name, Op::Colon, value) => {
					fields.iter().find(|(field, _)| *field == name.drop_meta().name()).map(|field| (name, value, field))
				}
				_ => None,
			};
			let (written, field) = match named {
				Some((_, value, field)) => (value.as_ref(), Some(field)),
				None => (argument, fields.get(index)),
			};
			let key = format!("{class}.{}", field.map_or("", |(name, _)| name.as_str()));
			let value = match self.field_value(&key, &key, field.map(|(_, held)| held), written) {
				Ok(value) => value,
				Err(stop) => return Some(Err(stop)),
			};
			lowered.push(match named {
				Some((name, ..)) => Node::Key(name.clone(), Op::Colon, Box::new(value)),
				None => value,
			});
		}
		Some(Ok((Node::List(lowered, Bracket::Round, crate::node::Separator::None), Signature::plain())))
	}

	/// `D(n inch)`, `D{diagonal: p.n inch}`: the unit after a value not written as a number is read as the next value, as
	/// units bind only after a number (notes/decisions.md); the error names the way that works
	fn unit_after_variable(&mut self, value: &Node, next: Option<&Node>) -> Option<Stop> {
		let unit = next.filter(|unit| self.unit_word(unit).is_some())?;
		match self.is_unbound_amount(value) {
			Ok(true) => Some(Stop::Error(format!("DimensionError: {}", super::unit_after_name_advice(value, unit)))),
			Ok(false) => None,
			Err(stop) => Some(stop),
		}
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
		(is_named_fields && matches!(name.drop_meta(), Node::Symbol(_)) && self.classes.contains_key(&class))
			.then_some((class, body))
	}

	fn named_construction_class(&self, node: &Node) -> Option<String> {
		self.named_construction_parts(node).map(|(class, _)| class)
	}

	/// `Run{distance: 5 km}`: each named field has its field's signature, as the arguments of `Run(5 km)` (card map-units)
	pub(super) fn named_construction(&mut self, node: &Node) -> Option<Result<(Node, Signature), Stop>> {
		let (class, body) = self.named_construction_parts(node)?;
		let fields = self.classes.get(&class)?.clone();
		let Node::List(entries, bracket, separator) = body.drop_meta() else {
			return None;
		};
		let mut lowered = vec![];
		for (index, entry) in entries.iter().enumerate() {
			let Node::Key(field, op, value) = entry.drop_meta() else {
				lowered.push(entry.clone());
				continue;
			};
			if let Some(stop) = self.unit_after_variable(value, entries.get(index + 1)) {
				return Some(Err(stop));
			}
			let held = fields.iter().find(|(name, _)| *name == field.drop_meta().name()).map(|(_, held)| held);
			let key = format!("{class}.{}", field.drop_meta().name());
			let value = match self.field_value(&key, &key, held, value) {
				Ok(value) => value,
				Err(stop) => return Some(Err(stop)),
			};
			lowered.push(Node::Key(field.clone(), *op, Box::new(value)));
		}
		Some(Ok((
			with_instance_body(node, Node::List(lowered, bracket.clone(), separator.clone())),
			Signature::plain(),
		)))
	}

	/// The class declaration with its unit fields declared numbers; a default value (`distance: km = 0 km`) has the unit
	pub(super) fn class_declaration(&mut self, name: Box<Node>, body: Box<Node>) -> Result<(Node, Signature), Stop> {
		let Some(fields) = self.classes.get(&name.drop_meta().name()).cloned() else {
			return Ok((Node::Type { name, body }, Signature::plain()));
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
			None => Ok((Node::Type { name, body: Box::new(body) }, Signature::plain())),
		}
	}

	fn numeric_field(&mut self, item: Node, fields: &Fields) -> Result<Node, Stop> {
		let unit_of = |field: &Node| {
			fields
				.iter()
				.find(|(name, signature)| *name == field.drop_meta().name() && !signature.is_empty())
				.map(|(_, signature)| signature.clone())
		};
		match item {
			Node::Meta { node, data } => Ok(Node::Meta { node: Box::new(self.numeric_field(*node, fields)?), data }),
			Node::Key(field, Op::Colon, field_type) if unit_of(&field).is_some() => {
				let number_type = |written: &Node| {
					let optional = if written.drop_meta().name().ends_with(OPTIONAL_MARK) {
						OPTIONAL_MARK.to_string()
					} else {
						String::new()
					};
					Box::new(Node::Symbol(format!("{NUMBER_WORD}{optional}")))
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
				let Some(held) = held else {
					return Ok(Node::Key(declaration, Op::Assign, default));
				};
				let (default, given) = self.infer(*default)?;
				if given != held {
					return Err(Stop::Error(format!(
						"DimensionError: the default of {} is {}, the field holds {}",
						declaration.serialize().trim(),
						shown(&given),
						shown(&held)
					)));
				}
				Ok(Node::Key(Box::new(self.numeric_field(*declaration, fields)?), Op::Assign, Box::new(default)))
			}
			other => Ok(other),
		}
	}

	/// `x.distance` of an instance static units cannot see, where distance is some class's unit field: loud
	pub(super) fn unseen_unit_field(&self, object: &Node, field: &Node) -> Option<Stop> {
		let field = field.drop_meta().name();
		let class = self
			.classes
			.iter()
			.find(|(_, fields)| fields.iter().any(|(name, signature)| *name == field && !signature.is_empty()))?
			.0;
		Some(Stop::Error(format!(
			"{}.{field}: the unit of {class}.{field} is known of an instance declared so: `x: {class} = …`, a constructor call, an element of a list declared [{class}]",
			object.serialize().trim()
		)))
	}
}
