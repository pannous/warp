//! Static units, stage 1 (notes/units_runtime.md): a program with quantities runs with plain exact numbers; a temperature's
//! amount counts from absolute zero, its unit's zero is added where it is written and taken off where it shows. Each expression's unit signature (dimension → power) is inferred
//! here and checked (`+ - ==` of different signatures is a DimensionError at compile time), each unit literal becomes its
//! amount in SI base units (`5 km` → 5000, `5 cm` → 1/20), and the program's final value is converted back to the finest
//! written unit after the run (`result_units` of the compile context). Stage 2: a function called with quantities (or whose body has unit
//! literals) is specialised per unit signature of its arguments, `speed·u0`. A quantity anywhere these stages do not
//! cover (recursion that changes units, a list added to a single value) leaves the program unchanged: the unit stays the loud undefined-variable error.
//! Stage 3: `print q`, `"text " + q` and a final value show the unit (`str(amount / unit) + "km"` at run time), `q as km`
//! and `q in km` check the dimension and choose the unit shown, `return q` carries the signature (all returns agree).
//! Stage 4: a list of quantities has its elements' signature one level deeper (`[1 m, 2 m]`, `[3, 4] cm`, list_units.rs):
//! `xs#i`, `for x in xs`, `.map` and `sum max min first last` give an element; `√q` halves the powers; `${q}` interpolation shows the unit.
//! Stage 6: a variable holding an object has one signature per field (`p = {dist: 0 m}`, `p.dist += 5 m`), a final object
//! names its fields' units in `warp.meta` (entry units); `x:any = q` keeps the signature, `x:km = q` checks it; `q.serialize()` is its
//! text; `xs.add(q)` checks the element signature.
//! Unit fields: `class Run{distance: km}` (unit_fields.rs), an instance's fields have signatures like an object's.

use super::{exponent_of, finest_units, unit_named, units_text, Dimension, Factor, Quantity, Unit};
use crate::lowering::function_head;
use crate::words::{MAP_WORD, PRINT_WORD, RETURN_WORD, STR_WORD};

mod function_units;
mod list_units;
use list_units::one_signature;

mod unit_fields;
use crate::declarations;
use crate::extensions::numbers::Number;
use crate::extensions::reals::Rational;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::collections::{HashMap, HashSet};
pub(crate) use unit_fields::unit_type;
pub(crate) use unit_fields::{lower_field_tolerances, remember_field_unit_words, si_quantity};

const LOOP_WORDS: [&str; 2] = ["for", "while"];
/// `speed·u0`: a function specialised for one unit signature of its arguments
const SPECIALISATION_SEPARATOR: &str = "·u";
/// A run-time amount as text that reads back: `1.5`, or a fraction `str` shows as `10/3` in parentheses, `(10/3)km/h`
const AMOUNT_TEXT: &str = r#"(if str(the_amount).contains("/") then "(" + str(the_amount) + ")" else str(the_amount))"#;
/// `print` rounds a fraction to two decimals, `6.17km`; `str` keeps it exact (P231)
const PRINTED_AMOUNT_TEXT: &str =
	r#"(if str(the_amount).contains("/") then str(round(the_amount * 100) / 100.0) else str(the_amount))"#;
const AMOUNT_PLACEHOLDER: &str = "the_amount";
/// Where a quantity's run-time text goes in the text of a whole object or list: `quantity_shown_0`
const QUANTITY_SHOWN: &str = "quantity_shown_";
/// `std_io("table", "insert", [r.distance, …])`: a host call takes SI amounts as they are (stored rows hold SI amounts)
const HOST_CALL: &str = "std_io";
/// `si_amount(q)`: the plain SI number of a quantity, so a library takes `0.5s` and `0.5` alike (lib/sound.warp)
const SI_AMOUNT_WORD: &str = "si_amount";
/// The text of a value: `str(q)`, and `text_form(q)` of `${q}` interpolation (interpolation.rs)
const TEXT_WORDS: [&str; 3] = [STR_WORD, "text_form", "serialize"];
/// List words that give an element: its signature (`mean` also spelled `average`, library_words.rs)
const ELEMENT_WORDS: [&str; 7] = ["sum", "max", "min", "first", "last", "mean", "average"];
/// `type(q)`: the type of a value, a quantity's is its dimension
const TYPE_WORD: &str = "type";

/// Dimension → power: `km/h` is length¹ time⁻¹
pub(super) type Powers = Vec<(Dimension, i32)>;

/// The units of an expression: each dimension's power, and how deep in lists its quantities are (`[3, 4] cm` is a
/// length at depth 1, a matrix of them at depth 2). A list of plain values stays plain, depth 0 (card dimensions-units Q3)
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(super) struct Signature {
	powers: Powers,
	depth: usize,
}

impl Signature {
	fn plain() -> Signature {
		Signature::default()
	}

	/// The signature of a list of values of this one
	fn list_of(&self) -> Signature {
		let depth = if self.powers.is_empty() { 0 } else { self.depth + 1 };
		Signature { powers: self.powers.clone(), depth }
	}

	/// The signature of an element of a list of this one
	fn element(&self) -> Signature {
		Signature { powers: self.powers.clone(), depth: self.depth.saturating_sub(1) }
	}

	fn is_list(&self) -> bool {
		self.depth > 0
	}

	/// These powers in lists `depth` deep
	fn nested(self, depth: usize) -> Signature {
		let depth = if self.powers.is_empty() { 0 } else { depth };
		Signature { depth, ..self }
	}
}

impl std::ops::Deref for Signature {
	type Target = [(Dimension, i32)];
	fn deref(&self) -> &Self::Target {
		&self.powers
	}
}

impl From<Vec<(Dimension, i32)>> for Signature {
	fn from(powers: Vec<(Dimension, i32)>) -> Signature {
		Signature { powers, depth: 0 }
	}
}

impl FromIterator<(Dimension, i32)> for Signature {
	fn from_iter<I: IntoIterator<Item = (Dimension, i32)>>(powers: I) -> Signature {
		Signature::from(powers.into_iter().collect::<Vec<_>>())
	}
}

type Fields = Vec<(String, Signature)>;

/// The units of a program's final value: of the value itself, or of the fields of the object it is
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ResultUnits {
	Whole(Vec<Factor>),
	Fields(Vec<(String, Vec<Factor>)>),
}

/// Why a program is no stage-1 program: `Unsupported` leaves it unchanged, `Error` is a compile error, `Recursive` is a
/// recursive call whose result signature is not known yet (the other branch of an if/else gives it)
enum Stop {
	Unsupported,
	Error(String),
	Recursive,
}

/// `si_amount(x)` is x once the units are checked: the calls output_form did not meet, or all of a program without units
fn without_si_amounts(node: Node) -> Node {
	match node.drop_meta() {
		Node::List(items, _, _)
			if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(name) if name == SI_AMOUNT_WORD) =>
		{
			without_si_amounts(items[1].clone())
		}
		_ => node.map_children(without_si_amounts),
	}
}

/// The program with unit literals as SI amounts and the units of its final value; None when it uses no units or uses
/// quantities beyond stage 1; Err for a dimension error
pub fn lower(program: &Node) -> Option<Result<Node, Node>> {
	match super::shadowing(program, || lower_shadowed(program)) {
		Some(lowered) => Some(lowered.map(without_si_amounts)),
		None => program.mentions(SI_AMOUNT_WORD).then(|| Ok(without_si_amounts(program.clone()))),
	}
}

fn lower_shadowed(program: &Node) -> Option<Result<Node, Node>> {
	let classes = unit_fields::unit_classes(program);
	if !super::needs_quantities(program) && classes.is_empty() {
		return None;
	}
	let mut inference = Inference::of(program, classes);
	let display = inference.display.clone();
	match inference.infer(program.clone()) {
		Ok((node, result)) => {
			// a final `q as km` shows km
			let shown_units = last_statement(program)
				.map(|last| conversion_target(last).unwrap_or_else(|| inference.shown_factors(last, &result)));
			let node = if inference.new_definitions.is_empty() {
				node
			} else {
				Node::List(
					[std::mem::take(&mut inference.new_definitions), vec![node]].concat(),
					Bracket::None,
					Separator::Semicolon,
				)
			};
			let units = match inference.result_fields.take() {
				Some(fields) => ResultUnits::Fields(
					fields
						.iter()
						.map(|(field, signature)| (field.clone(), display_factors(&display, signature)))
						.collect(),
				),
				None => ResultUnits::Whole(shown_units.unwrap_or_else(|| display_factors(&display, &result))),
			};
			let has_units = match &units {
				ResultUnits::Whole(factors) => !factors.is_empty(),
				ResultUnits::Fields(fields) => !fields.is_empty(),
			};
			crate::compiler::compile_context::change(|context| context.result_units = has_units.then_some(units));
			Some(Ok(node))
		}
		Err(Stop::Error(message)) => Some(Err(crate::node::error(&message))),
		Err(Stop::Unsupported | Stop::Recursive) => None,
	}
}

/// A constant expression of quantities, computed: its SI amount and signature (`2 s` is 2 of time, `90 min` 5400) or the
/// truth of a comparison (`1 km > 500 m`); None for an expression that is no constant
pub(super) fn constant(expression: &Node) -> Result<Option<(Node, Powers)>, String> {
	if expression.mentions_where(|name| unit_named(name).is_none()) {
		return Ok(None);
	}
	match Inference::of(expression, HashMap::new()).infer(expression.clone()) {
		Ok((value, signature)) if matches!(value.drop_meta(), Node::Number(_) | Node::True | Node::False) => {
			Ok(Some((value, signature.powers)))
		}
		Err(Stop::Error(message)) => Err(message),
		_ => Ok(None),
	}
}

/// The units written in a program, once per mention, with the units of its classes' fields
fn written_factors(program: &Node, classes: &HashMap<String, Fields>) -> Vec<Factor> {
	// a unit a constructor argument is written in (`Run(1500 m)`) is stored in its field's unit, which shows it
	let mut written = written_units(program);
	for stored in unit_fields::stored_argument_units(program, classes) {
		if let Some(at) = written.iter().position(|unit| std::ptr::eq(*unit, stored)) {
			written.remove(at);
		}
	}
	written.into_iter().map(|unit| Factor { unit, power: 1 }).chain(unit_fields::written_field_units(program)).collect()
}

impl Inference {
	/// The inference of a program: its written units show its quantities, its functions are specialised when called
	fn of(program: &Node, classes: HashMap<String, Fields>) -> Inference {
		let written = written_factors(program, &classes);
		let mut temperatures: Vec<&'static Unit> = vec![];
		for factor in &written {
			if factor.unit.dimension == Dimension::Temperature
				&& !temperatures.iter().any(|unit| std::ptr::eq(*unit, factor.unit))
			{
				temperatures.push(factor.unit)
			}
		}
		let mut definitions = HashMap::new();
		program.visit(&mut |node| {
			if let Some((name, definition)) = function_definition(node) {
				for arity in definition.required()..=definition.parameters.len() {
					definitions.insert((name.clone(), arity), definition.clone());
				}
			}
		});
		Inference {
			variables: HashMap::new(),
			globals: HashMap::new(),
			plain_lists: HashMap::new(),
			empty_lists: HashSet::new(),
			objects: HashMap::new(),
			classes,
			instances: HashMap::new(),
			class_lists: HashMap::new(),
			result_fields: None,
			definitions,
			specialised: HashMap::new(),
			in_progress: vec![],
			numbered: 0,
			new_definitions: vec![],
			display: finest_units(&written),
			written: HashMap::new(),
			temperatures,
			returns: vec![],
			at_top: true,
			in_host_call: false,
			started: HashMap::new(),
		}
	}
}

/// Every unit word written in `node`, once per mention
fn written_units(node: &Node) -> Vec<&'static Unit> {
	let mut written = vec![];
	node.visit(&mut |part| {
		if let Node::Symbol(name) = part {
			written.extend(unit_named(name))
		}
	});
	written
}

/// The units the last lowered program's final value has, once (None for a plain number)
pub(crate) fn take_result_units() -> Option<ResultUnits> {
	crate::compiler::compile_context::change(|context| context.result_units.take())
}

/// The entry of the module's `warp.meta` section (meta_section.rs) naming the units of `main`'s result (`km:1 h:-1`), or
/// of its result's fields (`dist=km:1;time=s:1`): a module that returns SI amounts says which quantities they are, and
/// running it reads them back (wasm_reader)
pub const UNITS_ENTRY: &str = "units";
const POWER_MARK: char = ':';
const FIELD_MARK: char = '=';
const FIELD_SEPARATOR: &str = ";";

fn factors_text(factors: &[Factor]) -> String {
	factors
		.iter()
		.map(|factor| format!("{}{POWER_MARK}{}", factor.unit.name, factor.power))
		.collect::<Vec<_>>()
		.join(" ")
}

fn factors_of(text: &str) -> Option<Vec<Factor>> {
	text.split(' ')
		.map(|factor| {
			let (name, power) = factor.split_once(POWER_MARK)?;
			Some(Factor { unit: unit_named(name)?, power: power.parse().ok()? })
		})
		.collect()
}

/// The module with the `units` entry of the units of its result, when the last lowered program's result has units
pub(crate) fn with_result_units(bytes: Vec<u8>) -> Vec<u8> {
	crate::formats::meta_section::with_entries(bytes, result_units_entry().into_iter().collect())
}

/// The `units` entry of the last lowered program's result, when it has units
pub(crate) fn result_units_entry() -> Option<(&'static str, Node)> {
	let text = match take_result_units()? {
		ResultUnits::Whole(factors) => factors_text(&factors),
		ResultUnits::Fields(fields) => fields
			.iter()
			.map(|(field, factors)| format!("{field}{FIELD_MARK}{}", factors_text(factors)))
			.collect::<Vec<_>>()
			.join(FIELD_SEPARATOR),
	};
	Some((UNITS_ENTRY, Node::Text(text)))
}

/// The result of running a module, as the quantity its `units` entry names (unchanged without it)
pub fn with_module_units(bytes: &[u8], result: Node) -> Node {
	match module_units(bytes) {
		Some(ResultUnits::Whole(units)) => quantity_of(result, &units),
		Some(ResultUnits::Fields(fields)) => with_field_units(result, &fields),
		None => result,
	}
}

fn module_units(bytes: &[u8]) -> Option<ResultUnits> {
	let Node::Text(text) = crate::formats::meta_section::entry(bytes, UNITS_ENTRY)? else {
		return None;
	};
	if !text.contains(FIELD_MARK) {
		return factors_of(&text).map(ResultUnits::Whole);
	}
	text.split(FIELD_SEPARATOR)
		.map(|entry| {
			let (field, factors) = entry.split_once(FIELD_MARK)?;
			Some((field.to_string(), factors_of(factors)?))
		})
		.collect::<Option<Vec<_>>>()
		.map(ResultUnits::Fields)
}

/// An object read back with the SI amounts of its quantity fields as the quantities they are (an object of one entry
/// reads back as that entry)
fn with_field_units(object: Node, fields: &[(String, Vec<Factor>)]) -> Node {
	match object {
		meta @ Node::Meta { .. } => meta.under_meta(|node| with_field_units(node, fields)),
		// an instance of a class with unit fields, the tag `Run{distance:3000}`
		Node::Key(class, Op::None, body) => Node::Key(class, Op::None, Box::new(with_field_units(*body, fields))),
		Node::List(entries, bracket, separator) => {
			Node::List(entries.into_iter().map(|entry| with_field_units(entry, fields)).collect(), bracket, separator)
		}
		Node::Key(field, op, value) => match fields.iter().find(|(name, _)| *name == field.name()) {
			Some((_, units)) => Node::Key(field, op, Box::new(quantity_of(*value, units))),
			None => Node::Key(field, op, value),
		},
		other => other,
	}
}

/// A run-time amount in SI base units as the quantity it is in `units`, a list of them as a list of quantities
pub(crate) fn quantity_of(value: Node, units: &[Factor]) -> Node {
	if let Node::List(items, bracket, separator) = value.drop_meta() {
		if *bracket == Bracket::Square {
			let quantities = items.iter().map(|item| quantity_of(item.clone(), units)).collect();
			return Node::List(quantities, bracket.clone(), separator.clone());
		}
	}
	let Some(per_unit) = super::scale(units, |factor| base_unit(factor.unit.dimension)).inverse() else {
		return value;
	};
	if let Some(amount) = exact_amount(&value) {
		return Node::data(Quantity {
			amount: amount.add(&shown_zero(units).neg()).mul(&per_unit),
			factors: units.to_vec(),
		});
	}
	let uncertain = match value.drop_meta() {
		Node::Data(data) => {
			data.downcast_ref::<crate::numbers::uncertain::Uncertain>().map(|amount| amount.scaled(per_unit.to_f64()))
		}
		_ => None,
	};
	uncertain.map_or(value, |amount| Node::data(super::UncertainQuantity::new(amount, units)))
}

pub(super) fn exact_amount(value: &Node) -> Option<Rational> {
	match value.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => exact_amount(&items[0]),
		Node::Number(Number::Int(n)) => Some(Rational::integer(*n)),
		Node::Number(Number::Quotient(n, d)) => Some(Rational::new(BigInt::from(*n), BigInt::from(*d))),
		// a written amount converts as the decimal it spells: 6.5km is 6500m, not 6499.999…m
		Node::Number(Number::Float(f)) if f.is_finite() => {
			let (numerator, denominator) = crate::wasm_emitter::numbers::exact::decimal_fraction(*f);
			Some(Rational::new(numerator, denominator))
		}
		_ => None,
	}
}

/// Dimension names as types of quantities (card dimensions-units), from lib/units.warp: `h: length`, `q is length²`,
/// `speed: length/duration`, and the derived names (`a: area = 3 m²`, Q4). The time dimension is `duration`, a subtype
/// of `time` (decision anchor-time), so `3 s is time` holds too
fn dimension_names() -> impl Iterator<Item = (&'static str, Vec<(Dimension, i32)>)> {
	let definitions = super::definitions::definitions();
	let named = definitions.dimensions.iter().map(|(name, dimension)| (*name, vec![(*dimension, 1)]));
	named.chain(definitions.derived_dimensions.iter().map(|(name, powers)| (*name, powers.clone())))
}

/// The base units a dimension name stands for: `length` → m, `speed` → m/s
fn named_dimension(name: &str) -> Option<Vec<Factor>> {
	let (_, powers) = dimension_names().find(|(known, _)| *known == name)?;
	Some(powers.iter().map(|(dimension, power)| Factor { unit: base_unit(*dimension), power: *power }).collect())
}

/// The name of a signature: `length` of m, `speed` of m/s, none of kg/m
fn dimension_name(signature: &Signature) -> Option<&'static str> {
	dimension_names()
		.map(|(name, _)| name)
		.find(|name| named_dimension(name).is_some_and(|factors| self::signature(&factors) == *signature))
}

/// The type of a quantity: its dimension's name, else its base units, in lists as deep as it is (`list of length`)
fn type_name(signature: &Signature) -> String {
	let element = signature.clone().nested(0);
	let dimension = dimension_name(&element).map_or_else(|| shown(&element), str::to_string);
	"list of ".repeat(signature.depth) + &dimension
}

/// A declared unit or dimension and how a DimensionError shows it: as written (`cm`), a dimension name with its base
/// units (`length (m)`, `area (m²)`), an expression of dimensions in base units (`m/s`)
struct Declaration {
	signature: Signature,
	shown: String,
}

fn declaration(declared: &Node, own_names: &HashMap<String, Signature>) -> Option<Declaration> {
	// `v: length[3]`, `w: cm[2]`: a list of quantities (the parser reads `[3]` as the index `#4`)
	if let Node::Key(element, Op::Hash, _) = declared.drop_meta() {
		let element = declaration(element, own_names)?;
		return Some(Declaration {
			signature: element.signature.list_of(),
			shown: format!("list of {}", element.shown),
		});
	}
	if let Some(units) = super::unit_expression(declared) {
		return Some(Declaration { signature: signature(&units), shown: declared.serialize().trim().to_string() });
	}
	let signature = dimension_signature(declared, own_names)?;
	let shown = match declared.drop_meta().symbol_name() {
		Some(dimension) => format!("{dimension} ({})", shown(&signature)),
		None => shown(&signature),
	};
	Some(Declaration { signature, shown })
}

/// `time` and `duration`, types of the time module too: a quantity of seconds is both, any other value's test stays
/// the time module's (`now is time`, `2 hours is duration`)
fn is_time_type(node: &Node, own_names: &HashMap<String, Signature>) -> bool {
	let duration = crate::time::TimeForm::Duration.name();
	[crate::time::TIME_WORD, duration]
		.into_iter()
		.any(|word| node.drop_meta().is_symbol(word) && !own_names.contains_key(word))
}

/// The signature a dimension expression names, `mass*length²`, unless the program names its words itself
fn dimension_signature(node: &Node, own_names: &HashMap<String, Signature>) -> Option<Signature> {
	let factors = super::factor_expression(node, &|leaf| {
		let name = leaf.drop_meta().symbol_name().filter(|name| !own_names.contains_key(*name))?;
		named_dimension(name)
	})?;
	Some(signature(&factors))
}

/// The base unit of a dimension (`m: length` in lib/units.warp), the unit amounts count in at run time
fn base_unit(dimension: Dimension) -> &'static Unit {
	let definitions = super::definitions::definitions();
	let (_, name) =
		definitions.base_units.iter().find(|(known, _)| *known == dimension).expect("a base unit per dimension");
	definitions.units.iter().find(|unit| unit.name == *name).expect("the base unit is a unit")
}

fn display_unit(display: &[&'static Unit], dimension: Dimension) -> &'static Unit {
	display.iter().find(|unit| unit.dimension == dimension).copied().unwrap_or_else(|| base_unit(dimension))
}

/// A signature in the units it shows in: the finest written unit of each dimension, in the order they are written
fn display_factors(display: &[&'static Unit], signature: &Signature) -> Vec<Factor> {
	let mut factors: Vec<Factor> = signature
		.iter()
		.map(|(dimension, power)| Factor { unit: display_unit(display, *dimension), power: *power })
		.collect();
	factors.sort_by_key(|factor| display.iter().position(|unit| unit.dimension == factor.unit.dimension));
	factors
}

/// A unit's amount in SI base units as a literal: km → 1000, cm → 1/20 of 5 cm
fn si_literal(unit: &'static Unit) -> Option<Node> {
	let amount = Rational::new(BigInt::from(unit.factor), BigInt::from(base_unit(unit.dimension).factor));
	number_node(&amount)
}

/// An amount in `unit` as an SI amount: `40 °C` is 40 + 273.15 (kelvin)
fn si_amount(amount: Node, unit: &'static Unit) -> Option<Node> {
	let scaled = folded(amount, Op::Mul, si_literal(unit)?);
	let zero = si_zero(unit);
	if zero.is_zero() {
		return Some(scaled);
	}
	match exact_amount(&scaled) {
		Some(amount) => number_node(&amount.add(&zero)),
		None => Some(Node::Key(Box::new(scaled), Op::Add, Box::new(number_node(&zero)?))),
	}
}

/// Where a unit's 0 lies in SI amounts: 273.15 for °C, 0 for a unit without a zero of its own
fn si_zero(unit: &Unit) -> Rational {
	Rational::new(BigInt::from(unit.zero), BigInt::from(base_unit(unit.dimension).factor))
}

/// The zero of the units a quantity shows in: only a lone temperature unit has one
fn shown_zero(units: &[Factor]) -> Rational {
	match units {
		[Factor { unit, power: 1 }] => si_zero(unit),
		_ => Rational::integer(0),
	}
}

/// `²`, `³` and `^ 2`: the power a quantity is raised to, a whole number written
fn written_power(op: Op, exponent: &Node) -> Option<i32> {
	match (op, exponent.drop_meta()) {
		(Op::Square | Op::Cube, Node::Empty) => Some(exponent_of(op)),
		(Op::Pow, Node::Number(Number::Int(power))) => i32::try_from(*power).ok(),
		_ => None,
	}
}

fn number_node(amount: &Rational) -> Option<Node> {
	let numerator = amount.numerator.to_i64()?;
	Some(match amount.denominator.to_i64()? {
		1 => Node::int(numerator),
		denominator => Node::Number(Number::Quotient(numerator, denominator)),
	})
}

/// The signature of units: `km/h` and `m/s` alike
fn signature(factors: &[Factor]) -> Signature {
	super::signature(factors).into()
}

fn combined(left: &[(Dimension, i32)], right: &[(Dimension, i32)], sign: i32) -> Signature {
	let factors: Vec<Factor> = left
		.iter()
		.map(|(d, p)| (*d, *p))
		.chain(right.iter().map(|(d, p)| (*d, p * sign)))
		.map(|(dimension, power)| Factor { unit: base_unit(dimension), power })
		.collect();
	signature(&factors)
}

fn is_number_type(declared: &Node) -> bool {
	crate::analyzer::builtin_type_kind(&declared.drop_meta().name())
		.is_some_and(|kind| matches!(kind, crate::type_kinds::Kind::Int | crate::type_kinds::Kind::Float))
}

fn shown(signature: &Signature) -> String {
	let factors: Vec<Factor> =
		signature.iter().map(|(dimension, power)| Factor { unit: base_unit(*dimension), power: *power }).collect();
	match (factors.is_empty(), signature.is_list()) {
		(true, _) => "a plain number".to_string(),
		(false, true) => type_name(signature),
		(false, false) => units_text(&factors),
	}
}

/// A type word with a value kind (analyzer::type_word_kind) or one a run-time test checks (type_tests::runtime_kind_mask)
fn is_kinded_or_testable_type(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if crate::analyzer::type_word_kind(name).is_some() || crate::type_tests::runtime_kind_mask(name).is_some())
}

/// The variable a declaration names: `x` of `x = 5m` and of a bare `x`
fn declared_name(declaration: &Node) -> Option<String> {
	match declaration.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(target, Op::Assign | Op::Define, _) => declared_name(target),
		_ => None,
	}
}

fn dimension_error(op: Op, left: &Signature, right: &Signature) -> Stop {
	Stop::Error(format!("DimensionError: {} {op} {}: incompatible units", shown(left), shown(right)))
}

/// A user function `name(params) := body`: its parameter names and body
#[derive(Clone)]
struct Definition {
	parameters: Vec<String>,
	/// `b: Bus`: each parameter as its specialisation declares it, typed unless by a unit (its body's units are resolved)
	declared: Vec<Node>,
	/// `t = 1s`: what a call leaving the parameter out gives
	defaults: Vec<Option<Node>>,
	body: Node,
}

impl Definition {
	/// The fewest arguments a call gives: the parameters before the trailing defaults
	fn required(&self) -> usize {
		self.defaults.iter().rposition(Option::is_none).map_or(0, |last| last + 1)
	}
}

fn function_definition(node: &Node) -> Option<(String, Definition)> {
	let definition =
		function_head(node).filter(|definition| definition.is_glued_head() && definition.plain_operator().is_some())?;
	let (parameters, (declared, defaults)) = definition
		.written_parameters
		.iter()
		.map(|parameter| match parameter.drop_meta() {
			Node::Symbol(name) => Some((name.clone(), (Node::Symbol(name.clone()), None))),
			Node::Key(name, Op::Colon, type_node) if !mentions_units(type_node) => {
				Some((name.name(), (parameter.drop_meta().clone(), None)))
			}
			Node::Key(name, Op::Colon, _) => Some((name.name(), (Node::Symbol(name.name()), None))),
			// `t = 1s`: a call leaving t out gives the default
			Node::Key(name, Op::Assign, default) => {
				Some((name.name(), (Node::Symbol(name.name()), Some(default.as_ref().clone()))))
			}
			_ => None,
		})
		.collect::<Option<(Vec<_>, (Vec<_>, Vec<_>))>>()?;
	Some((definition.name.to_string(), Definition { parameters, declared, defaults, body: definition.body.clone() }))
}

/// Does the code use a unit word: not as a field name (`q.s` of a field s)
fn mentions_units(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(name) => unit_named(name).is_some(),
		Node::Key(object, Op::Dot, _) => mentions_units(object),
		Node::Key(left, _, right) => mentions_units(left) || mentions_units(right),
		// the first word of a list is a call or command (`min(n, 3)`, `use m`), no unit
		Node::List(items, _, _) => match items.split_first() {
			Some((head, rest)) => {
				(!matches!(head.drop_meta(), Node::Symbol(_)) && mentions_units(head))
					|| rest.iter().any(mentions_units)
			}
			None => false,
		},
		_ => false,
	}
}

/// A specialisation: the function and the unit signatures of its arguments
type Specialisation = (String, Vec<Signature>);

struct InProgress {
	key: Specialisation,
	name: String,
	recursive: bool,
	result: Option<Signature>,
}

struct Inference {
	variables: HashMap<String, Signature>,
	/// Main-level variables declared `global`: a function writing one keeps its signature (card quantity-global)
	globals: HashMap<String, Signature>,
	/// Variables holding a list of plain values and how deep it nests (`xs cm` makes it a list of quantities); a list of
	/// quantities is known by its variable's signature
	plain_lists: HashMap<String, usize>,
	/// Variables holding an empty list (`ø`, `[]`): the first list appended or assigned gives its signature
	empty_lists: HashSet<String>,
	/// Variables holding an object with quantity fields: each field (in written order) and its signature, none for a plain one
	objects: HashMap<String, Fields>,
	/// The classes with unit fields, the variables holding an instance of one and the lists of one (unit_fields.rs)
	classes: HashMap<String, Fields>,
	instances: HashMap<String, String>,
	class_lists: HashMap<String, String>,
	/// The quantity fields of the program's final value, when it is such an object
	result_fields: Option<Fields>,
	/// by name and parameter count: `tone(p, d) := tone(p, d, "sine")` overloads by arity (overloads.rs)
	definitions: HashMap<(String, usize), Definition>,
	/// Specialised functions: their name and result signature
	specialised: HashMap<Specialisation, (String, Signature)>,
	/// The specialisations being inferred: their name, whether they call themselves, and their result signature once a
	/// first pass found it
	in_progress: Vec<InProgress>,
	numbered: usize,
	new_definitions: Vec<Node>,
	/// The finest written unit of each dimension: the unit a quantity shows in unless converted
	display: Vec<&'static Unit>,
	/// The units written in each variable's values: the units it shows in (card print-print)
	written: HashMap<String, Vec<Factor>>,
	/// The temperature units written, each once
	temperatures: Vec<&'static Unit>,
	/// The signatures of the `return`s of each function being specialised
	returns: Vec<Vec<Signature>>,
	/// Inferring the program's own statements (its last one may be a whole list of quantities)
	at_top: bool,
	/// Inferring the arguments of a host call
	in_host_call: bool,
	/// The specialisation each task-started function runs as: `go f()` starts `f·u0`, `await job` reads its result
	started: HashMap<String, String>,
}

/// The last statement of a program or block
fn last_statement(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		Node::List(items, _, Separator::Semicolon | Separator::Newline) => items.last().and_then(last_statement),
		other => Some(other),
	}
}

/// The units a conversion shows: `q as km`, `q in km/h`
fn conversion_target(node: &Node) -> Option<Vec<Factor>> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => conversion_target(&items[0]),
		other => super::conversion(other).map(|(_, units)| units),
	}
}

/// The quantity in `quantity` (an SI amount at run time) as text in `units`: `str(amount / 1000) + "km"`, a fraction in parentheses
fn quantity_text(quantity: Node, units: &[Factor], amount_text: &str) -> Option<Node> {
	let per_unit = number_node(&super::scale(units, |factor| base_unit(factor.unit.dimension)))?;
	let zero = shown_zero(units);
	let above_zero = match zero.is_zero() {
		true => quantity,
		false => Node::Key(Box::new(quantity), Op::Sub, Box::new(number_node(&zero)?)),
	};
	let amount = Node::Key(Box::new(above_zero), Op::Div, Box::new(per_unit));
	let text =
		crate::lowering::library_words::substitute(crate::warp_parser::parse(amount_text), AMOUNT_PLACEHOLDER, &amount);
	Some(Node::Key(Box::new(text), Op::Add, Box::new(Node::Text(super::unit_suffix(units)))))
}

/// `left op right`, computed when both are numbers: `5 km` is the literal 5000, a value and no computation (blocks.rs reads
/// `dist: 5000` as data, `dist: 5*1000` as a block)
fn folded(left: Node, op: Op, right: Node) -> Node {
	// a sum or comparison of floats stays as the run computes it: 0.1 + 0.2 is no 3/10
	let exact = |node: &Node| match node.drop_meta() {
		Node::Number(Number::Float(_)) => None,
		_ => exact_amount(node),
	};
	let amount = match (exact_amount(&left), exact_amount(&right), op) {
		(Some(a), Some(b), Op::Mul) => Some(a.mul(&b)),
		(Some(a), Some(b), Op::Div) => b.inverse().map(|inverse| a.mul(&inverse)),
		_ => match (exact(&left), exact(&right)) {
			(Some(a), Some(b)) if op == Op::Add => Some(a.add(&b)),
			(Some(a), Some(b)) if op == Op::Sub => Some(a.add(&b.neg())),
			(Some(a), Some(b)) if op.is_comparison() => return Node::from(holds(a.compare(&b), op)),
			_ => None,
		},
	};
	amount.as_ref().and_then(number_node).unwrap_or_else(|| Node::Key(Box::new(left), op, Box::new(right)))
}

fn holds(order: std::cmp::Ordering, comparison: Op) -> bool {
	match comparison {
		Op::Eq => order.is_eq(),
		Op::Ne => order.is_ne(),
		Op::Lt => order.is_lt(),
		Op::Gt => order.is_gt(),
		Op::Le => order.is_le(),
		_ => order.is_ge(),
	}
}

/// `serialize()`, `add(q)`: the method word and arguments of a method call `object.method(…)`
fn method_of(call: &Node) -> Option<(&str, &[Node])> {
	let Node::List(items, Bracket::Round, _) = call.drop_meta() else {
		return None;
	};
	let (head, arguments) = items.split_first()?;
	let Node::Symbol(method) = head.drop_meta() else {
		return None;
	};
	(TEXT_WORDS.contains(&method.as_str()) || crate::list_mutations::appends(method, arguments.len()))
		.then_some((method.as_str(), arguments))
}

/// The entries of an object literal `{dist: 5 m, name: "run"}`: field and value each
fn object_entries(value: &Node) -> Option<Vec<(Node, Op, Node)>> {
	let Node::List(entries, Bracket::Curly, _) = value.drop_meta() else {
		return None;
	};
	entries
		.iter()
		.map(|entry| match entry.drop_meta() {
			Node::Key(field, op @ (Op::Colon | Op::Assign), value) if matches!(field.drop_meta(), Node::Symbol(_)) => {
				Some((field.as_ref().clone(), *op, value.as_ref().clone()))
			}
			_ => None,
		})
		.collect()
}

impl Inference {
	fn infer(&mut self, node: Node) -> Result<(Node, Signature), Stop> {
		if let Some((_, definition)) = function_definition(&node) {
			// a body or default with unit literals compiles only in its specialisations
			let kept = if self.has_units(&definition) { Node::Empty } else { node };
			return Ok((kept, Signature::plain()));
		}
		if let Some(construction) = self.named_construction(&node) {
			return construction;
		}
		// `function min` is the function, no minute
		if crate::closures::referenced_function(&node).is_some() {
			return Ok((node, Signature::plain()));
		}
		match node {
			Node::Meta { node, data } => {
				let (node, signature) = self.infer(*node)?;
				Ok((Node::Meta { node: Box::new(node), data }, signature))
			}
			// an instance of a class is passed whole as any value: its fields hold SI amounts wherever they go
			Node::Symbol(name) if self.objects.contains_key(&name) && !self.instances.contains_key(&name) => {
				Err(Stop::Unsupported)
			}
			Node::Type { name, body } => self.class_declaration(name, body),
			Node::Symbol(name) => match (self.variables.get(&name), unit_named(&name)) {
				(Some(signature), _) => Ok((Node::Symbol(name.clone()), signature.clone())),
				// a run-time amount counts from the base unit's zero, which °C and °F do not share
				(None, Some(unit)) if unit.zero != 0 => Err(Stop::Error(format!(
					"a temperature in {} is computed at compile time only, not at run time yet",
					unit.name
				))),
				(None, Some(unit)) => {
					Ok((si_literal(unit).ok_or(Stop::Unsupported)?, Signature::from(vec![(unit.dimension, 1)])))
				}
				(None, None) => Ok((Node::Symbol(name), Signature::plain())),
			},
			Node::Key(target, op @ (Op::Assign | Op::Define), value) => self.assignment(*target, op, *value),
			Node::Key(keyword, Op::Colon, declaration) if super::is_global_keyword(&keyword) => {
				let (declaration, signature) = self.infer(*declaration)?;
				if let (true, Some(name)) = (self.in_progress.is_empty(), declared_name(&declaration)) {
					self.globals.insert(name, signature.clone());
				}
				Ok((Node::Key(keyword, Op::Colon, Box::new(declaration)), signature))
			}
			Node::Key(target, op, value) if op.is_compound_assign() => {
				self.remember_written(&target, &value);
				let (target, held) = self.infer(*target)?;
				let (value, given) = self.infer(*value)?;
				// `xs = ø; xs += [3 cm]`: an empty list takes the units of what is appended first
				if op.base_op() == Op::Add && given.is_list() && self.empty_lists.remove(&target.name()) {
					self.variables.insert(target.name(), given.clone());
					return Ok((Node::Key(Box::new(target), op, Box::new(value)), given));
				}
				let result = self.arithmetic(op.base_op(), &held, &given)?;
				if result != held {
					return Err(dimension_error(op, &held, &given));
				}
				Ok((Node::Key(Box::new(target), op, Box::new(value)), held))
			}
			Node::Key(quantity, Op::As, unit) if super::unit_expression(&unit).is_some() => {
				self.converted(*quantity, &unit)
			}
			// `"distance " + d`: the quantity joins as its text, `"distance " + (d as km)` in km
			Node::Key(left, Op::Add, right) if super::joins_text(&left) || super::joins_text(&right) => {
				let (left_units, right_units) = (conversion_target(&left), conversion_target(&right));
				let (left, left_signature) = self.infer(*left)?;
				let (right, right_signature) = self.infer(*right)?;
				let left = self.as_text(left, &left_signature, left_units, AMOUNT_TEXT)?;
				let right = self.as_text(right, &right_signature, right_units, AMOUNT_TEXT)?;
				Ok((Node::Key(Box::new(left), Op::Add, Box::new(right)), Signature::plain()))
			}
			// `q is length`: the dimension of a quantity, known when compiled (card dimensions-units)
			Node::Key(left, op @ (Op::Eq | Op::Ne), right)
				if dimension_signature(&right, &self.variables).is_some() || is_time_type(&right, &self.variables) =>
			{
				let named =
					dimension_signature(&right, &self.variables).unwrap_or_else(|| vec![(Dimension::Time, 1)].into());
				let (_, given) = self.infer(left.as_ref().clone())?;
				if given.is_empty() && is_time_type(&right, &self.variables) {
					return self.unchecked_key(*left, op, *right);
				}
				Ok((Node::from((given == named) == (op == Op::Eq)), Signature::plain()))
			}
			// `r.distance == ø` of an optional unit field: ø has no dimension, any value may be it; `x is list` asks its type
			Node::Key(left, op @ (Op::Eq | Op::Ne), right)
				if left.is_empty_node() || right.is_empty_node() || is_kinded_or_testable_type(&right) =>
			{
				self.unchecked_key(*left, op, *right)
			}
			// `2 m * "a"`: a text repeats a plain count of times (card units-text-repeat)
			Node::Key(left, Op::Mul, right) if left.is_text_literal() || right.is_text_literal() => {
				let (left, left_signature) = self.infer(*left)?;
				let (right, right_signature) = self.infer(*right)?;
				let count = combined(&left_signature, &right_signature, 1);
				match count.is_empty() {
					true => Ok((Node::Key(Box::new(left), Op::Mul, Box::new(right)), Signature::plain())),
					false => Err(Stop::Error(format!(
						"DimensionError: a text repeats a plain number of times, is given {}",
						shown(&count)
					))),
				}
			}
			// `[3, 4] cm` is `[3 cm, 4 cm]`: a unit after a list is each element's (dimensions-units Q3)
			Node::Key(list, op @ (Op::Mul | Op::Div), unit)
				if matches!(list.drop_meta(), Node::List(_, Bracket::Square, _)) && self.is_unit_after_value(&unit) =>
			{
				self.infer(list_units::each_in_unit(&list, op, &unit))
			}
			Node::Key(amount, Op::Mul, unit) if self.unit_word(&unit).is_some() => {
				self.amount_in(*amount, self.unit_word(&unit).expect("guarded"))
			}
			Node::Key(left, op, right) if matches!(op, Op::Add | Op::Sub | Op::Mul | Op::Div) || op.is_comparison() => {
				let (left_depth, right_depth) = (self.plain_list_depth(&left), self.plain_list_depth(&right));
				let (left, left_signature) = self.infer(*left)?;
				let (right, right_signature) = self.infer(*right)?;
				let quantities = !left_signature.is_empty() || !right_signature.is_empty();
				let (left_depth, right_depth) =
					(left_depth.max(left_signature.depth), right_depth.max(right_signature.depth));
				if quantities && matches!(op, Op::Mul | Op::Div) && (left_depth > 0 || right_depth > 0) {
					return self.each_combined(
						(left, &left_signature, left_depth),
						op,
						(right, &right_signature, right_depth),
					);
				}
				let signature = self.arithmetic(op, &left_signature, &right_signature)?;
				let computed = self.with_one_zero(folded(left, op, right), op, &signature)?;
				Ok((computed, signature))
			}
			// `-3m`: the quantity keeps its units; `-20°C` counts from the zero of °C
			Node::Key(nothing, Op::Neg, operand) if nothing.is_empty_node() => {
				let (operand, signature) = self.infer(*operand)?;
				let negated = match exact_amount(&operand) {
					Some(amount) => number_node(&amount.neg()).ok_or(Stop::Unsupported)?,
					None => Node::Key(nothing, Op::Neg, Box::new(operand)),
				};
				let twice_zero = self.temperature_zero(&signature).add(&self.temperature_zero(&signature));
				match twice_zero.is_zero() {
					true => Ok((negated, signature)),
					false => {
						let zero = number_node(&twice_zero).ok_or(Stop::Unsupported)?;
						Ok((folded(zero, Op::Add, negated), signature))
					}
				}
			}
			Node::Key(base, op, exponent) if written_power(op, &exponent).is_some() => {
				let power = written_power(op, &exponent).expect("guarded");
				let (base, signature) = self.infer(*base)?;
				let signature =
					signature.iter().map(|(dimension, base_power)| (*dimension, base_power * power)).collect();
				Ok((Node::Key(Box::new(base), op, exponent), signature))
			}
			// `if c then a else b`: both branches have one signature
			Node::Key(if_then, Op::Else, otherwise) if matches!(if_then.drop_meta(), Node::Key(_, Op::Then, _)) => {
				let (if_then, then_signature, otherwise, else_signature) =
					match (self.infer(*if_then.clone()), self.infer(*otherwise.clone())) {
						// a recursive branch has the base case's units (the specialisation infers its body again to check that)
						(Err(Stop::Recursive), Ok((otherwise, signature))) => {
							(*if_then, signature.clone(), otherwise, signature)
						}
						(Ok((if_then, signature)), Err(Stop::Recursive)) => {
							(if_then, signature.clone(), *otherwise, signature)
						}
						(then_inferred, else_inferred) => {
							let (if_then, then_signature) = then_inferred?;
							let (otherwise, else_signature) = else_inferred?;
							(if_then, then_signature, otherwise, else_signature)
						}
					};
				if then_signature != else_signature {
					return Err(Stop::Error(format!(
						"DimensionError: the branches give {} and {}",
						shown(&then_signature),
						shown(&else_signature)
					)));
				}
				Ok((Node::Key(Box::new(if_then), Op::Else, Box::new(otherwise)), then_signature))
			}
			Node::Key(
				left,
				op @ (Op::Then | Op::If | Op::Do | Op::While | Op::Not | Op::And | Op::Or | Op::Range | Op::To),
				right,
			) => self.unchecked_key(*left, op, *right),
			// `xs#2`, `xs[1]`: an element of a list of quantities
			// `#xs`: the count of a list
			Node::Key(nothing, Op::Hash, list) if nothing.is_empty_node() => {
				let (list, _) = self.infer(*list)?;
				Ok((Node::Key(nothing, Op::Hash, Box::new(list)), Signature::plain()))
			}
			Node::Key(list, Op::Hash, index) => {
				let (list, signature) = self.infer(*list)?;
				let (index, index_signature) = self.infer(*index)?;
				if !index_signature.is_empty() || (!signature.is_empty() && !signature.is_list()) {
					return Err(Stop::Unsupported);
				}
				Ok((Node::Key(Box::new(list), Op::Hash, Box::new(index)), signature.element()))
			}
			// `√a`: the powers halve, an odd power has no root
			Node::Key(nothing, Op::Sqrt, value) if matches!(nothing.drop_meta(), Node::Empty) => {
				let (value, signature) = self.infer(*value)?;
				if signature.iter().any(|(_, power)| power % 2 != 0) {
					return Err(Stop::Error(format!(
						"DimensionError: √ of {} has no unit (an odd power)",
						shown(&signature)
					)));
				}
				let signature = signature.iter().map(|(dimension, power)| (*dimension, power / 2)).collect();
				Ok((Node::Key(nothing, Op::Sqrt, Box::new(value)), signature))
			}
			Node::Key(object, Op::Dot, field) if self.field_signature(&object, &field).is_some() => {
				let signature = self.field_signature(&object, &field).expect("guarded");
				// `Run(5 km).distance`: the constructor call's quantities lowered too
				let object = match object.drop_meta() {
					Node::Symbol(_) => *object,
					_ => self.infer(*object)?.0,
				};
				Ok((Node::Key(Box::new(object), Op::Dot, field), signature))
			}
			Node::Key(object, Op::Dot, call) if method_of(&call).is_some() => self.method_call(*object, *call),
			Node::Key(source, Op::Dot, call) if map_function(&call).is_some() => self.mapped_list(*source, *call),
			// `q.s`: a field name is no unit
			Node::Key(object, Op::Dot, field) => {
				if let Some(unseen) = self.unseen_unit_field(&object, &field) {
					return Err(unseen);
				}
				let (object, signature) = self.infer(*object)?;
				if !signature.is_empty() {
					return Err(Stop::Unsupported);
				}
				Ok((Node::Key(Box::new(object), Op::Dot, field), Signature::plain()))
			}
			Node::Key(left, op, right) => self.plain_key(*left, op, *right),
			Node::List(items, bracket, separator) => self.infer_list(items, bracket, separator),
			other => Ok((other, Signature::plain())),
		}
	}

	/// `q as km`, `q in km`: the dimension must match; the amount stays in SI units, the unit shown is chosen where it
	/// is shown
	fn converted(&mut self, quantity: Node, unit: &Node) -> Result<(Node, Signature), Stop> {
		let target = signature(&super::unit_expression(unit).ok_or(Stop::Unsupported)?);
		let (quantity, given) = self.infer(quantity)?;
		// a list converts each element: `[3, 4] cm in m`
		if *given != *target {
			return Err(Stop::Error(format!(
				"DimensionError: {} cannot be converted to {}",
				shown(&given),
				shown(&target)
			)));
		}
		Ok((quantity, given))
	}

	/// A key whose sides' units need not match: the value of `c then q` has q's signature, any other none
	fn unchecked_key(&mut self, left: Node, op: Op, right: Node) -> Result<(Node, Signature), Stop> {
		let (left, _) = self.infer(left)?;
		let (right, signature) = self.infer(right)?;
		let signature = if op == Op::Then { signature } else { Signature::plain() };
		Ok((Node::Key(Box::new(left), op, Box::new(right)), signature))
	}

	/// Any other key: no quantity may take part (stage 1 does not know what it does with the unit)
	fn plain_key(&mut self, left: Node, op: Op, right: Node) -> Result<(Node, Signature), Stop> {
		let (left, left_signature) = self.infer(left)?;
		let (right, right_signature) = self.infer(right)?;
		if !left_signature.is_empty() || !right_signature.is_empty() {
			return Err(Stop::Unsupported);
		}
		Ok((Node::Key(Box::new(left), op, Box::new(right)), Signature::plain()))
	}

	/// `x = q`, `x:any = q` (keeps the signature), `x:km = q` (checks it: annotated code is strict, P203), `p.dist = q`
	fn assignment(&mut self, target: Node, op: Op, value: Node) -> Result<(Node, Signature), Stop> {
		match target.drop_meta() {
			Node::Symbol(name) => self.assign_variable(name.clone(), target, op, value, None),
			Node::Key(name, Op::Colon, declared) if matches!(name.drop_meta(), Node::Symbol(_)) => {
				match declaration(declared, &self.variables) {
					// a unit or dimension is no type the emitter knows: the variable is a plain number with this signature
					Some(declaration) => {
						self.assign_variable(name.name(), name.as_ref().clone(), op, value, Some(declaration))
					}
					// `x: int = 1 m`: a number type holds plain numbers only (card units-annotation)
					None if is_number_type(declared) => {
						let plain = Declaration { signature: Signature::plain(), shown: shown(&Signature::plain()) };
						self.assign_variable(name.name(), target.clone(), op, value, Some(plain))
					}
					None => {
						self.declared_instance(&name.name(), declared);
						match self.instances.contains_key(&name.name()) {
							true => self.infer(value).map(|(value, _)| {
								(Node::Key(Box::new(target.clone()), op, Box::new(value)), Signature::plain())
							}),
							false => self.assign_variable(name.name(), target.clone(), op, value, None),
						}
					}
				}
			}
			Node::Key(object, Op::Dot, field) if self.field_signature(object, field).is_some() => {
				let held = self.field_signature(object, field).expect("guarded");
				let class = self.instance_class(object).unwrap_or_default();
				let key = format!("{class}.{}", field.drop_meta().name());
				let value = self.field_value(target.serialize().trim(), &key, Some(&held), &value)?;
				Ok((Node::Key(Box::new(target), op, Box::new(value)), held))
			}
			_ => self.plain_key(target, op, value),
		}
	}

	fn assign_variable(
		&mut self,
		name: String,
		target: Node,
		op: Op,
		value: Node,
		declared: Option<Declaration>,
	) -> Result<(Node, Signature), Stop> {
		if let Some(instance) = self.assigned_instance(&name, &target, op, &value) {
			return instance;
		}
		// `xs = [3, 4]`: `xs cm` is a list of quantities
		match self.plain_list_depth(&value) {
			0 => self.plain_lists.remove(&name),
			depth => self.plain_lists.insert(name.clone(), depth),
		};
		// `q = p`: an alias has the signatures of what it names
		if let Node::Symbol(other) = value.drop_meta() {
			if let Some(fields) = self.objects.get(other).cloned() {
				self.objects.insert(name, fields);
				return Ok((Node::Key(Box::new(target), op, Box::new(value)), Signature::plain()));
			}
		}
		// an object literal and a plain value show in the units written in them; a list of class instances (above) in its
		// fields' declared units
		self.remember_written(&target, &value);
		let is_empty_list = value.is_empty_node()
			|| matches!(value.drop_meta(), Node::List(items, Bracket::Square, _) if items.is_empty());
		if let Some(object) = self.object_literal(&value) {
			let (object, fields) = object?;
			self.objects.insert(name, fields);
			return Ok((Node::Key(Box::new(target), op, Box::new(object)), Signature::plain()));
		}
		let (value, signature) = self.infer(value)?;
		if let Some(declared) = declared.filter(|declared| declared.signature != signature) {
			let declared = declared.shown;
			let given = match dimension_name(&signature) {
				Some(dimension) => format!("{} ({dimension})", shown(&signature)),
				None => shown(&signature),
			};
			return Err(Stop::Error(format!("DimensionError: {name} is declared {declared}, is given {given}")));
		}
		// an empty list fits any list, and the list given next fits it
		let was_empty = self.empty_lists.remove(&name);
		if is_empty_list {
			self.empty_lists.insert(name.clone());
		}
		let fits = is_empty_list || was_empty;
		if let Some(earlier) = self.variables.get(&name).filter(|earlier| **earlier != signature && !fits) {
			return Err(Stop::Error(super::reassigned_dimension(
				&name,
				&shown(earlier),
				earlier.is_empty(),
				&shown(&signature),
			)));
		}
		self.variables.insert(name, signature.clone());
		Ok((Node::Key(Box::new(target), op, Box::new(value)), signature))
	}

	/// The units written in an expression and in the values of the variables it reads
	fn written_in(&self, node: &Node) -> Vec<Factor> {
		let mut written = vec![];
		node.visit(&mut |part| {
			if let Node::Symbol(name) = part {
				match self.written.get(name) {
					Some(units) => written.extend(units.iter().cloned()),
					None => written.extend(unit_named(name).map(|unit| Factor { unit, power: 1 })),
				}
			}
		});
		written
	}

	/// `x = value`, `x += value`: x shows in the units written in its values
	fn remember_written(&mut self, target: &Node, value: &Node) {
		let written = self.written_in(value);
		self.written.entry(target.name()).or_default().extend(written);
	}

	/// A signature in the units an expression shows in: the finest written in it, else in the program
	fn shown_factors(&self, node: &Node, signature: &Signature) -> Vec<Factor> {
		let mut display = finest_units(&self.written_in(node));
		let unwritten: Vec<_> = (self.display.iter())
			.filter(|unit| !display.iter().any(|shown| shown.dimension == unit.dimension))
			.copied()
			.collect();
		display.extend(unwritten);
		display_factors(&display, signature)
	}

	/// `{dist: 5 m, name: "run"}`: the object with its amounts in SI units and its fields' signatures; None for an object
	/// without quantities
	fn object_literal(&mut self, value: &Node) -> Option<Result<(Node, Fields), Stop>> {
		let entries = object_entries(value)?;
		let mut lowered = vec![];
		let mut fields = vec![];
		for (field, op, value) in entries {
			let (value, signature) = match self.infer(value) {
				Ok(inferred) => inferred,
				Err(stop) => return Some(Err(stop)),
			};
			fields.push((field.name(), signature));
			lowered.push(Node::Key(Box::new(field), op, Box::new(value)));
		}
		if fields.iter().all(|(_, signature)| signature.is_empty()) {
			return None;
		}
		let Node::List(_, bracket, separator) = value.drop_meta() else {
			return None;
		};
		Some(Ok((Node::List(lowered, bracket.clone(), separator.clone()), fields)))
	}

	/// `p.dist` of an object with quantity fields, or of an instance of a class with unit fields: the field's signature
	/// (none for a plain field)
	fn field_signature(&self, object: &Node, field: &Node) -> Option<Signature> {
		let Node::Symbol(field) = field.drop_meta() else {
			return None;
		};
		let fields = match object.drop_meta() {
			Node::Symbol(object) if self.objects.contains_key(object) => self.objects.get(object)?.clone(),
			_ => self.instance_fields(object)?,
		};
		Some(fields.iter().find(|(name, _)| name == field).map(|(_, signature)| signature.clone()).unwrap_or_default())
	}

	/// `q.serialize()`: the quantity's text; `xs.add(q)`: q has the element signature of xs
	fn method_call(&mut self, object: Node, call: Node) -> Result<(Node, Signature), Stop> {
		let (method, arguments) = method_of(&call).expect("a method call");
		if let ([argument], Node::Symbol(list)) = (arguments, object.drop_meta()) {
			let held =
				self.variables.get(list).filter(|held| held.is_list() && crate::list_mutations::appends(method, 1));
			if let Some(element) = held.map(Signature::element) {
				let (argument, given) = self.infer(argument.clone())?;
				if given != element {
					return Err(Stop::Error(format!(
						"DimensionError: {list} holds {}, is given {}",
						shown(&element),
						shown(&given)
					)));
				}
				let call =
					Node::List(vec![Node::Symbol(method.to_string()), argument], Bracket::Round, Separator::None);
				return Ok((Node::Key(Box::new(object), Op::Dot, Box::new(call)), Signature::plain()));
			}
		}
		let is_text = TEXT_WORDS.contains(&method) && arguments.is_empty();
		let (object, signature) = self.infer(object)?;
		if is_text && !signature.is_empty() {
			return Ok((self.as_text(object, &signature, None, AMOUNT_TEXT)?, Signature::plain()));
		}
		let (call, _) = self.infer(call)?;
		if !signature.is_empty() {
			return Err(Stop::Unsupported);
		}
		Ok((Node::Key(Box::new(object), Op::Dot, Box::new(call)), Signature::plain()))
	}

	/// The signature of a computation of two values. Lists of one signature append (`xs + [5 m]`) and compare; a list
	/// and a single value are ambiguous in `+ -` and comparisons, and multiply element by element (each_combined)
	fn arithmetic(&self, op: Op, left: &Signature, right: &Signature) -> Result<Signature, Stop> {
		match op {
			Op::Add | Op::Eq | Op::Ne if left.is_list() && right.is_list() && left != right => {
				Err(dimension_error(op, left, right))
			}
			Op::Add if left.is_list() && right.is_list() => Ok(left.clone()),
			Op::Eq | Op::Ne if left.is_list() && right.is_list() => Ok(Signature::plain()),
			_ if left.is_list() != right.is_list() && !matches!(op, Op::Mul | Op::Div) => Err(Stop::Error(format!(
				"{} {op} {}: append [the quantity] or map the list to {op} each element",
				type_name(left),
				type_name(right)
			))),
			_ if left.is_list() || right.is_list() => Err(Stop::Unsupported),
			Op::Mul => Ok(combined(left, right, 1)),
			Op::Div => Ok(combined(left, right, -1)),
			_ if left != right => Err(dimension_error(op, left, right)),
			_ if op.is_comparison() => Ok(Signature::plain()),
			_ => Ok(left.clone()),
		}
	}

	/// `xs * 2`, `xs / 1 s`, `2 * xs` of one list (`depth` deep) and one value: each element computed; adding,
	/// comparing or combining two lists is no stage of these (Unsupported)
	fn each_combined(
		&self,
		(left, left_signature, left_depth): (Node, &Signature, usize),
		op: Op,
		(right, right_signature, right_depth): (Node, &Signature, usize),
	) -> Result<(Node, Signature), Stop> {
		if !matches!(op, Op::Mul | Op::Div) || (left_depth > 0 && right_depth > 0) {
			return Err(Stop::Unsupported);
		}
		let signature = self.arithmetic(op, &left_signature.clone().nested(0), &right_signature.clone().nested(0))?;
		let depth = left_depth.max(right_depth);
		let each = match left_depth > 0 {
			true => list_units::each_element(left, depth, &|element| folded(element, op, right.clone())),
			false => list_units::each_element(right, depth, &|element| folded(left.clone(), op, element)),
		};
		Ok((each, signature.nested(depth)))
	}

	/// `template` with each `quantity_shown_<i>` the run-time text of the quantity at the source `amounts[i]` (SI amounts):
	/// `str(amount / (1/100)) + " cm"`. Substituted as nodes: a scale like 5/18 has no source form of its own
	fn with_quantity_texts(
		&self,
		template: &str,
		amounts: &[(String, &Signature)],
		amount_text: &str,
	) -> Result<Node, Stop> {
		let mut text = crate::warp_parser::parse_generated(template);
		for (index, (amount, signature)) in amounts.iter().enumerate() {
			let shown = self.as_text(crate::warp_parser::parse_generated(amount), signature, None, amount_text)?;
			text = crate::lowering::library_words::substitute(text, &format!("{QUANTITY_SHOWN}{index}"), &shown);
		}
		Ok(text)
	}

	/// `km` in `5 km` and `5km`: a unit word no variable shadows
	fn unit_word(&self, node: &Node) -> Option<&'static Unit> {
		match node.drop_meta() {
			Node::Symbol(name) if !self.variables.contains_key(name) => unit_named(name),
			_ => None,
		}
	}

	/// `40°C`: an amount in a unit as an SI amount, counted from the unit's zero
	fn amount_in(&mut self, amount: Node, unit: &'static Unit) -> Result<(Node, Signature), Stop> {
		let plain_depth = self.plain_list_depth(&amount);
		let (amount, given) = self.infer(amount)?;
		let signature = combined(&given, &[(unit.dimension, 1)], 1);
		// `xs cm` of a list: each element in the unit
		let depth = plain_depth.max(given.depth);
		if depth > 0 {
			let scale = si_literal(unit).filter(|_| unit.zero == 0).ok_or(Stop::Unsupported)?;
			let each = list_units::each_element(amount, depth, &|element| folded(element, Op::Mul, scale.clone()));
			return Ok((each, signature.nested(depth)));
		}
		Ok((si_amount(amount, unit).ok_or(Stop::Unsupported)?, signature))
	}

	/// The zero of the unit a temperature shows in, 0 for any other signature
	fn temperature_zero(&self, signature: &Signature) -> Rational {
		match &**signature {
			[(Dimension::Temperature, 1)] if !signature.is_list() => {
				si_zero(display_unit(&self.display, Dimension::Temperature))
			}
			_ => Rational::integer(0),
		}
	}

	/// A sum of temperatures counts one zero, of the one unit they are written in: 20 °C + 5 °C is 25 °C
	fn with_one_zero(&self, computed: Node, op: Op, signature: &Signature) -> Result<Node, Stop> {
		let zero = self.temperature_zero(signature);
		if !matches!(op, Op::Add | Op::Sub) || zero.is_zero() {
			return Ok(computed);
		}
		if let [first, second, ..] = self.temperatures.as_slice() {
			return Err(Stop::Error(format!(
				"DimensionError: temperatures add in one unit only, given {} and {}",
				first.name, second.name
			)));
		}
		let undo = if op == Op::Add { Op::Sub } else { Op::Add };
		Ok(folded(computed, undo, number_node(&zero).ok_or(Stop::Unsupported)?))
	}

	/// A whole object with quantity fields as text, built at run time: `{dist:500 m name:"run"}`. The quantity fields come
	/// first, the plain ones follow as an object of them shows them
	fn object_text(&self, name: &str, amount_text: &str) -> Result<Node, Stop> {
		let fields = self.objects.get(name).ok_or(Stop::Unsupported)?;
		let mut quantities = vec![];
		let mut amounts = vec![];
		let mut plain = vec![];
		for (field, signature) in fields {
			match signature.is_empty() {
				true => plain.push(format!("{field}: {name}.{field}")),
				false => {
					quantities.push(format!("\"{field}:\" + {QUANTITY_SHOWN}{}", amounts.len()));
					amounts.push((format!("{name}.{field}"), signature));
				}
			}
		}
		let rest = match plain.is_empty() {
			true => "\"}\"".to_string(),
			false => format!("\" \" + text_form({{{}}})[1..]", plain.join(", ")),
		};
		self.with_quantity_texts(
			&format!("\"{{\" + {} + {rest}", quantities.join(" + \" \" + ")),
			&amounts,
			amount_text,
		)
	}

	/// The text of a whole object with quantities (`print p`, `"${p}"`); None for any other value
	fn object_shown(&mut self, value: &Node, amount_text: &str) -> Option<Result<Node, Stop>> {
		match value.drop_meta() {
			Node::Symbol(name) if self.objects.contains_key(name) => Some(self.object_text(name, amount_text)),
			_ => None,
		}
	}

	/// A value as it joins a text: a quantity as its text in `units` (else in the finest written units), a list of them
	/// as the list's text, anything else as it is
	fn as_text(
		&self,
		value: Node,
		signature: &Signature,
		units: Option<Vec<Factor>>,
		amount_text: &str,
	) -> Result<Node, Stop> {
		if signature.is_empty() {
			return Ok(value);
		}
		let units = units.unwrap_or_else(|| display_factors(&self.display, signature));
		if signature.is_list() {
			return list_units::list_text(value, signature.depth, &units, amount_text);
		}
		quantity_text(value, &units, amount_text).ok_or(Stop::Unsupported)
	}

	/// The items of a list literal of quantities and their signature; None for a list of plain values
	/// A list word's arguments and the signature it gives: an element of a list (`sum(xs)`), the one signature of its
	/// quantities (`max(a, b)`), a plain count (`count xs`); None for any other word
	fn list_word(&mut self, word: &str, arguments: &[Node]) -> Option<Result<(Vec<Node>, Signature), Stop>> {
		let counting = crate::analyzer::is_counting_word(word);
		if !ELEMENT_WORDS.contains(&word) && !counting {
			return None;
		}
		Some((|| {
			let (arguments, signatures) = self.infer_all(arguments)?;
			let signature = match signatures.as_slice() {
				_ if counting => Signature::plain(),
				[list] if list.is_list() => list.element(),
				_ => one_signature(&signatures)?,
			};
			Ok((arguments, signature))
		})())
	}

	/// Each node lowered, and its signature
	fn infer_all(&mut self, nodes: &[Node]) -> Result<(Vec<Node>, Vec<Signature>), Stop> {
		let mut lowered = vec![];
		let mut signatures = vec![];
		for node in nodes {
			let (node, signature) = self.infer(node.clone())?;
			lowered.push(node);
			signatures.push(signature);
		}
		Ok((lowered, signatures))
	}

	/// `runs.map(r => r.distance)`, `xs.map(d => d * 2)` (or an `it` body) of a list of a class, of quantities or of
	/// plain values: its function's parameter is an element, the result a list of what the body gives
	fn mapped_list(&mut self, source: Node, call: Node) -> Result<(Node, Signature), Stop> {
		let (method, function, separator) = map_function(&call).ok_or(Stop::Unsupported)?;
		let (parameter, body) = match function.drop_meta() {
			Node::Key(parameter, Op::Arrow, body) => (parameter.drop_meta().name(), body.as_ref().clone()),
			body => (crate::words::IMPLICIT_PARAMETER.to_string(), body.clone()),
		};
		// a table's list reads as `runs·load()`
		let source_name = match source.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			loaded => crate::database_tables::loaded_table(loaded),
		};
		let class = source_name.and_then(|name| self.class_lists.get(&name).cloned());
		let source = match class {
			Some(class) => {
				self.declare_instance(parameter, class);
				source
			}
			None => {
				let (source, signature) = self.infer(source)?;
				self.variables.insert(parameter, signature.element());
				source
			}
		};
		let (body, element) = self.infer(body)?;
		let function = match function.drop_meta() {
			Node::Key(parameter, Op::Arrow, _) => Node::Key(parameter.clone(), Op::Arrow, Box::new(body)),
			_ => body,
		};
		let call = Node::List(vec![method, function], Bracket::Round, separator);
		Ok((Node::Key(Box::new(source), Op::Dot, Box::new(call)), element.list_of()))
	}
}

/// The method word, function and separator of a call `.map(f)`
fn map_function(call: &Node) -> Option<(Node, Node, Separator)> {
	let Node::List(items, Bracket::Round, separator) = call.drop_meta() else {
		return None;
	};
	match items.as_slice() {
		[method, function] if method.drop_meta().name() == MAP_WORD => {
			Some((method.clone(), function.clone(), separator.clone()))
		}
		_ => None,
	}
}
