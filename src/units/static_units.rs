//! Static units, stage 1 (notes/units_runtime.md): a program with quantities that `units::answer` cannot evaluate at compile
//! time (loops, branches) runs with plain exact numbers. Each expression's unit signature (dimension → power) is inferred
//! here and checked (`+ - ==` of different signatures is a DimensionError at compile time), each unit literal becomes its
//! amount in SI base units (`5 km` → 5000, `5 cm` → 1/20), and the program's final value is converted back to the finest
//! written unit after the run (`RESULT_UNITS`). Stage 2: a function called with quantities (or whose body has unit
//! literals) is specialised per unit signature of its arguments, `speed·u0`. A quantity anywhere these stages do not
//! cover (lists, interpolation, recursion) leaves the program unchanged: the unit stays the loud undefined-variable error.
//! Stage 3: `print q`, `"text " + q` and a final value show the unit (`str(amount / unit) + "km"` at run time), `q as km`
//! and `q in km` check the dimension and choose the unit shown, `return q` carries the signature (all returns agree).
//! Stage 4: a variable holding a list of quantities has one element signature (`xs = [1 m, 2 m]`), used by `xs#i`,
//! `sum max min first last count` and `for x in xs`; `√q` halves the powers; `${q}` interpolation shows the unit.
//! Stage 6: a variable holding an object has one signature per field (`p = {dist: 0 m}`, `p.dist += 5 m`), a final object
//! names its fields' units in `warp.meta` (entry units); `x:any = q` keeps the signature, `x:km = q` checks it; `q.serialize()` is its
//! text; `xs.add(q)` checks the element signature.
//! Unit fields: `class Run{distance: km}` (unit_fields.rs), an instance's fields have signatures like an object's.

use super::{finest_units, signature, unit_named, units_text, Dimension, Factor, Quantity, Unit, UNITS};

mod unit_fields;
pub(crate) use unit_fields::{lower_field_tolerances, si_quantity};
pub(crate) use unit_fields::unit_type;
use crate::extensions::numbers::Number;
use crate::extensions::reals::Rational;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::cell::RefCell;
use std::collections::HashMap;

/// The SI base unit of each dimension, the unit amounts count in at run time
const BASE_UNITS: [(Dimension, &str); 4] = [(Dimension::Length, "m"), (Dimension::Mass, "kg"), (Dimension::Time, "s"), (Dimension::Era, "AD")];
const LOOP_WORDS: [&str; 2] = ["for", "while"];
/// `speed·u0`: a function specialised for one unit signature of its arguments
const SPECIALISATION_SEPARATOR: &str = "·u";
const TEXT_WORD: &str = "str";
/// A run-time amount as text that reads back: `1.5`, or a fraction `str` shows as `10/3` in parentheses, `(10/3)km/h`
const AMOUNT_TEXT: &str = r#"(if str(the_amount).contains("/") then "(" + str(the_amount) + ")" else str(the_amount))"#;
/// `print` rounds a fraction to two decimals, `6.17km`; `str` keeps it exact (P231)
const PRINTED_AMOUNT_TEXT: &str = r#"(if str(the_amount).contains("/") then str(round(the_amount * 100) / 100.0) else str(the_amount))"#;
const AMOUNT_PLACEHOLDER: &str = "the_amount";
const PRINT_WORD: &str = "print";
/// `std_io("table", "insert", [r.distance, …])`: a host call takes SI amounts as they are (stored rows hold SI amounts)
const HOST_CALL: &str = "std_io";
const RETURN_WORD: &str = "return";
/// The text of a value: `str(q)`, and `text_form(q)` of `${q}` interpolation (interpolation.rs)
const TEXT_WORDS: [&str; 3] = [TEXT_WORD, "text_form", "serialize"];
/// `xs.add(q)`: list methods that take an element
const ELEMENT_METHODS: [&str; 2] = ["add", "push"];
/// `xs.map(f)`: a list of what f gives each element
const MAP_WORD: &str = "map";
/// List words that give an element: its signature (`mean` also spelled `average`, library_words.rs)
const ELEMENT_WORDS: [&str; 7] = ["sum", "max", "min", "first", "last", "mean", "average"];
/// List words that give a plain number
const COUNT_WORDS: [&str; 3] = ["count", "size", "length"];

type Signature = Vec<(Dimension, i32)>;
type Fields = Vec<(String, Signature)>;

/// The units of a program's final value: of the value itself, or of the fields of the object it is
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ResultUnits {
	Whole(Vec<Factor>),
	Fields(Vec<(String, Vec<Factor>)>),
}

thread_local! {
	/// The units of the final value of the last lowered program, to convert its run-time amount back (pipeline::eval_program)
	static RESULT_UNITS: RefCell<Option<ResultUnits>> = const { RefCell::new(None) };
}

/// Why a program is no stage-1 program: `Unsupported` leaves it unchanged, `Error` is a compile error
enum Stop {
	Unsupported,
	Error(String),
}

/// The program with unit literals as SI amounts and the units of its final value; None when it uses no units or uses
/// quantities beyond stage 1; Err for a dimension error
pub fn lower(program: &Node) -> Option<Result<Node, Node>> {
	super::shadowing(program, || lower_shadowed(program))
}

fn lower_shadowed(program: &Node) -> Option<Result<Node, Node>> {
	let classes = unit_fields::unit_classes(program);
	if !super::needs_quantities(program) && classes.is_empty() {
		return None;
	}
	// a unit a constructor argument is written in (`Run(1500 m)`) is stored in its field's unit, which shows it
	let mut written = written_units(program);
	for stored in unit_fields::stored_argument_units(program, &classes) {
		if let Some(at) = written.iter().position(|unit| std::ptr::eq(*unit, stored)) {
			written.remove(at);
		}
	}
	let written = written.into_iter().map(|unit| Factor { unit, power: 1 }).chain(unit_fields::written_field_units(program)).collect::<Vec<_>>();
	let display = finest_units(&written);
	let mut definitions = HashMap::new();
	program.visit(&mut |node| {
		if let Some((name, definition)) = function_definition(node) {
			definitions.insert(name, definition);
		}
	});
	let mut inference = Inference { variables: HashMap::new(), lists: HashMap::new(), objects: HashMap::new(), classes, instances: HashMap::new(), class_lists: HashMap::new(), result_fields: None, definitions, specialised: HashMap::new(), in_progress: vec![], new_definitions: vec![], display: display.clone(), returns: vec![], at_top: true, in_host_call: false };
	match inference.infer(program.clone()) {
		Ok((node, result)) => {
			// a final `q as km` shows km
			let shown_units = last_statement(program).and_then(conversion_target);
			let node = if inference.new_definitions.is_empty() {
				node
			} else {
				Node::List([std::mem::take(&mut inference.new_definitions), vec![node]].concat(), Bracket::None, Separator::Semicolon)
			};
			let units = match inference.result_fields.take() {
				Some(fields) => ResultUnits::Fields(fields.iter().map(|(field, signature)| (field.clone(), display_factors(&display, signature))).collect()),
				None => ResultUnits::Whole(shown_units.unwrap_or_else(|| display_factors(&display, &result))),
			};
			let has_units = match &units { ResultUnits::Whole(factors) => !factors.is_empty(), ResultUnits::Fields(fields) => !fields.is_empty() };
			RESULT_UNITS.with(|cell| *cell.borrow_mut() = has_units.then_some(units));
			Some(Ok(node))
		}
		Err(Stop::Error(message)) => Some(Err(crate::node::error(&message))),
		Err(Stop::Unsupported) => None,
	}
}

/// Every unit word written in `node`, once per mention
fn written_units(node: &Node) -> Vec<&'static Unit> {
	let mut written = vec![];
	node.visit(&mut |part| if let Node::Symbol(name) = part { written.extend(unit_named(name)) });
	written
}

/// The units the last lowered program's final value has, once (None for a plain number)
pub(crate) fn take_result_units() -> Option<ResultUnits> {
	RESULT_UNITS.with(|cell| cell.borrow_mut().take())
}

/// The entry of the module's `warp.meta` section (meta_section.rs) naming the units of `main`'s result (`km:1 h:-1`), or
/// of its result's fields (`dist=km:1;time=s:1`): a module that returns SI amounts says which quantities they are, and
/// running it reads them back (wasm_reader)
pub const UNITS_ENTRY: &str = "units";
const POWER_MARK: char = ':';
const FIELD_MARK: char = '=';
const FIELD_SEPARATOR: &str = ";";

fn factors_text(factors: &[Factor]) -> String {
	factors.iter().map(|factor| format!("{}{POWER_MARK}{}", factor.unit.name, factor.power)).collect::<Vec<_>>().join(" ")
}

fn factors_of(text: &str) -> Option<Vec<Factor>> {
	text.split(' ').map(|factor| {
		let (name, power) = factor.split_once(POWER_MARK)?;
		Some(Factor { unit: unit_named(name)?, power: power.parse().ok()? })
	}).collect()
}

/// The module with the `units` entry of the units of its result, when the last lowered program's result has units
pub(crate) fn with_result_units(bytes: Vec<u8>) -> Vec<u8> {
	crate::meta_section::with_entries(bytes, result_units_entry().into_iter().collect())
}

/// The `units` entry of the last lowered program's result, when it has units
pub(crate) fn result_units_entry() -> Option<(&'static str, Node)> {
	let text = match take_result_units()? {
		ResultUnits::Whole(factors) => factors_text(&factors),
		ResultUnits::Fields(fields) => fields.iter().map(|(field, factors)| format!("{field}{FIELD_MARK}{}", factors_text(factors))).collect::<Vec<_>>().join(FIELD_SEPARATOR),
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
	let Node::Text(text) = crate::meta_section::entry(bytes, UNITS_ENTRY)? else { return None };
	if !text.contains(FIELD_MARK) {
		return factors_of(&text).map(ResultUnits::Whole);
	}
	text.split(FIELD_SEPARATOR).map(|entry| {
		let (field, factors) = entry.split_once(FIELD_MARK)?;
		Some((field.to_string(), factors_of(factors)?))
	}).collect::<Option<Vec<_>>>().map(ResultUnits::Fields)
}

/// An object read back with the SI amounts of its quantity fields as the quantities they are (an object of one entry
/// reads back as that entry)
fn with_field_units(object: Node, fields: &[(String, Vec<Factor>)]) -> Node {
	match object {
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_field_units(*node, fields)), data },
		// an instance of a class with unit fields, the tag `Run{distance:3000}`
		Node::Key(class, Op::None, body) => Node::Key(class, Op::None, Box::new(with_field_units(*body, fields))),
		Node::List(entries, bracket, separator) => Node::List(entries.into_iter().map(|entry| with_field_units(entry, fields)).collect(), bracket, separator),
		Node::Key(field, op, value) => match fields.iter().find(|(name, _)| *name == field.name()) {
			Some((_, units)) => Node::Key(field, op, Box::new(quantity_of(*value, units))),
			None => Node::Key(field, op, value),
		},
		other => other,
	}
}

/// A run-time amount in SI base units as the quantity it is in `units`
pub(crate) fn quantity_of(value: Node, units: &[Factor]) -> Node {
	let Some(per_unit) = super::scale(units, |factor| base_unit(factor.unit.dimension)).inverse() else { return value };
	if let Some(amount) = exact_amount(&value) {
		return Node::data(Quantity { amount: amount.mul(&per_unit), factors: units.to_vec() });
	}
	let uncertain = match value.drop_meta() {
		Node::Data(data) => data.downcast_ref::<crate::uncertain::Uncertain>().map(|amount| amount.scaled(per_unit.to_f64())),
		_ => None,
	};
	uncertain.map_or(value, |amount| Node::data(super::UncertainQuantity::new(amount, units)))
}

fn exact_amount(value: &Node) -> Option<Rational> {
	match value.drop_meta() {
		Node::Number(Number::Int(n)) => Some(Rational::integer(*n)),
		Node::Number(Number::Quotient(n, d)) => Some(Rational::new(BigInt::from(*n), BigInt::from(*d))),
		Node::Number(Number::Float(f)) if Number::is_exact_decimal(*f) => {
			let (numerator, denominator) = crate::wasm_emitter::exact::decimal_fraction(*f);
			Some(Rational::new(numerator, denominator))
		}
		_ => None,
	}
}

fn base_unit(dimension: Dimension) -> &'static Unit {
	let name = BASE_UNITS.iter().find(|(known, _)| *known == dimension).map(|(_, name)| *name).expect("a base unit per dimension");
	UNITS.iter().find(|unit| unit.name == name).expect("the base unit is a unit")
}

fn display_unit(display: &[&'static Unit], dimension: Dimension) -> &'static Unit {
	display.iter().find(|unit| unit.dimension == dimension).copied().unwrap_or_else(|| base_unit(dimension))
}

/// A signature in the units it shows in: the finest written unit of each dimension
fn display_factors(display: &[&'static Unit], signature: &Signature) -> Vec<Factor> {
	signature.iter().map(|(dimension, power)| Factor { unit: display_unit(display, *dimension), power: *power }).collect()
}

/// A unit's amount in SI base units as a literal: km → 1000, cm → 1/20 of 5 cm
fn si_literal(unit: &'static Unit) -> Option<Node> {
	let amount = Rational::new(BigInt::from(unit.factor), BigInt::from(base_unit(unit.dimension).factor));
	number_node(&amount)
}

fn number_node(amount: &Rational) -> Option<Node> {
	let numerator = amount.numerator.to_i64()?;
	Some(match amount.denominator.to_i64()? {
		1 => Node::int(numerator),
		denominator => Node::Number(Number::Quotient(numerator, denominator)),
	})
}

fn combined(left: &Signature, right: &Signature, sign: i32) -> Signature {
	let factors: Vec<Factor> = left.iter().map(|(d, p)| (*d, *p)).chain(right.iter().map(|(d, p)| (*d, p * sign)))
		.map(|(dimension, power)| Factor { unit: base_unit(dimension), power })
		.collect();
	signature(&factors)
}

fn is_text(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Text(_) | Node::Char(_))
}

fn is_number_type(declared: &Node) -> bool {
	crate::analyzer::builtin_type_kind(&declared.drop_meta().name()).is_some_and(|kind| matches!(kind, crate::type_kinds::Kind::Int | crate::type_kinds::Kind::Float))
}

fn shown(signature: &Signature) -> String {
	let factors: Vec<Factor> = signature.iter().map(|(dimension, power)| Factor { unit: base_unit(*dimension), power: *power }).collect();
	if factors.is_empty() { "a plain number".to_string() } else { units_text(&factors) }
}

/// ø, which has no dimension: an optional unit field may hold it
fn is_nothing(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Empty)
}

fn dimension_error(op: Op, left: &Signature, right: &Signature) -> Stop {
	Stop::Error(format!("DimensionError: {} {op} {}: the units do not match", shown(left), shown(right)))
}

/// A user function `name(params) := body`: its parameter names and body
#[derive(Clone)]
struct Definition {
	parameters: Vec<String>,
	body: Node,
}

fn function_definition(node: &Node) -> Option<(String, Definition)> {
	let Node::Key(head, Op::Define | Op::Assign, body) = node.drop_meta() else { return None };
	let Node::List(items, Bracket::Round, Separator::None) = head.drop_meta() else { return None };
	let (name, parameters) = items.split_first()?;
	let Node::Symbol(name) = name.drop_meta() else { return None };
	let parameters = parameters.iter().map(|parameter| match parameter.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(name, Op::Colon, _) => Some(name.name()),
		_ => None,
	}).collect::<Option<Vec<_>>>()?;
	Some((name.clone(), Definition { parameters, body: body.as_ref().clone() }))
}

/// Does the code use a unit word: not as a field name (`q.s` of a field s)
fn mentions_units(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(name) => unit_named(name).is_some(),
		Node::Key(object, Op::Dot, _) => mentions_units(object),
		Node::Key(left, _, right) => mentions_units(left) || mentions_units(right),
		// the first word of a list is a call or command (`min(n, 3)`, `use m`), no unit
		Node::List(items, _, _) => match items.split_first() {
			Some((head, rest)) => (!matches!(head.drop_meta(), Node::Symbol(_)) && mentions_units(head)) || rest.iter().any(mentions_units),
			None => false,
		},
		_ => false,
	}
}

/// A specialisation: the function and the unit signatures of its arguments
type Specialisation = (String, Vec<Signature>);

struct Inference {
	variables: HashMap<String, Signature>,
	/// Variables holding a list of quantities: the signature of its elements
	lists: HashMap<String, Signature>,
	/// Variables holding an object with quantity fields: each field (in written order) and its signature, none for a plain one
	objects: HashMap<String, Fields>,
	/// The classes with unit fields, the variables holding an instance of one and the lists of one (unit_fields.rs)
	classes: HashMap<String, Fields>,
	instances: HashMap<String, String>,
	class_lists: HashMap<String, String>,
	/// The quantity fields of the program's final value, when it is such an object
	result_fields: Option<Fields>,
	definitions: HashMap<String, Definition>,
	/// Specialised functions: their name and result signature
	specialised: HashMap<Specialisation, (String, Signature)>,
	in_progress: Vec<Specialisation>,
	new_definitions: Vec<Node>,
	/// The finest written unit of each dimension: the unit a quantity shows in unless converted
	display: Vec<&'static Unit>,
	/// The signatures of the `return`s of each function being specialised
	returns: Vec<Vec<Signature>>,
	/// Inferring the program's own statements (its last one may be a whole list of quantities)
	at_top: bool,
	/// Inferring the arguments of a host call
	in_host_call: bool,
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
	let amount = Node::Key(Box::new(quantity), Op::Div, Box::new(per_unit));
	let text = crate::lowering::library_words::substitute(crate::warp_parser::parse(amount_text), AMOUNT_PLACEHOLDER, &amount);
	Some(Node::Key(Box::new(text), Op::Add, Box::new(Node::Text(super::unit_suffix(units)))))
}

/// `left op right`, computed when both are numbers: `5 km` is the literal 5000, a value and no computation (blocks.rs reads
/// `dist: 5000` as data, `dist: 5*1000` as a block)
fn folded(left: Node, op: Op, right: Node) -> Node {
	let amount = match (exact_amount(&left), exact_amount(&right), op) {
		(Some(a), Some(b), Op::Mul) => Some(a.mul(&b)),
		(Some(a), Some(b), Op::Div) => b.inverse().map(|inverse| a.mul(&inverse)),
		_ => None,
	};
	amount.as_ref().and_then(number_node).unwrap_or_else(|| Node::Key(Box::new(left), op, Box::new(right)))
}

/// `serialize()`, `add(q)`: the method word and arguments of a method call `object.method(…)`
fn method_of(call: &Node) -> Option<(&str, &[Node])> {
	let Node::List(items, Bracket::Round, _) = call.drop_meta() else { return None };
	let (head, arguments) = items.split_first()?;
	let Node::Symbol(method) = head.drop_meta() else { return None };
	(TEXT_WORDS.contains(&method.as_str()) || ELEMENT_METHODS.contains(&method.as_str())).then_some((method.as_str(), arguments))
}

/// The entries of an object literal `{dist: 5 m, name: "run"}`: field and value each
fn object_entries(value: &Node) -> Option<Vec<(Node, Op, Node)>> {
	let Node::List(entries, Bracket::Curly, _) = value.drop_meta() else { return None };
	entries.iter().map(|entry| match entry.drop_meta() {
		Node::Key(field, op @ (Op::Colon | Op::Assign), value) if matches!(field.drop_meta(), Node::Symbol(_)) => Some((field.as_ref().clone(), *op, value.as_ref().clone())),
		_ => None,
	}).collect()
}

impl Inference {
	fn infer(&mut self, node: Node) -> Result<(Node, Signature), Stop> {
		if let Some((_, definition)) = function_definition(&node) {
			// a body with unit literals compiles only in its specialisations
			let kept = if mentions_units(&definition.body) { Node::Empty } else { node };
			return Ok((kept, vec![]));
		}
		if let Some(construction) = self.named_construction(&node) {
			return construction;
		}
		match node {
			Node::Meta { node, data } => {
				let (node, signature) = self.infer(*node)?;
				Ok((Node::Meta { node: Box::new(node), data }, signature))
			}
			// a list of quantities only through its elements (`xs#i`, `sum(xs)`, `for x in xs`): printed whole it would show SI amounts
			// an instance of a class is passed whole as any value: its fields hold SI amounts wherever they go
			Node::Symbol(name) if self.lists.contains_key(&name) || (self.objects.contains_key(&name) && !self.instances.contains_key(&name)) => Err(Stop::Unsupported),
			Node::Type { name, body } => self.class_declaration(name, body),
			Node::Symbol(name) => match (self.variables.get(&name), unit_named(&name)) {
				(Some(signature), _) => Ok((Node::Symbol(name.clone()), signature.clone())),
				(None, Some(unit)) => Ok((si_literal(unit).ok_or(Stop::Unsupported)?, vec![(unit.dimension, 1)])),
				(None, None) => Ok((Node::Symbol(name), vec![])),
			},
			Node::Key(target, op @ (Op::Assign | Op::Define), value) => self.assignment(*target, op, *value),
			Node::Key(target, op, value) if op.is_compound_assign() => {
				let (target, held) = self.infer(*target)?;
				let (value, given) = self.infer(*value)?;
				let result = self.arithmetic(op.base_op(), &held, &given)?;
				if result != held {
					return Err(dimension_error(op, &held, &given));
				}
				Ok((Node::Key(Box::new(target), op, Box::new(value)), held))
			}
			// `q as km`: the dimension must match; the amount stays in SI units, the unit shown is chosen where it is shown
			Node::Key(quantity, Op::As, unit) if super::unit_expression(&unit).is_some() => {
				let target = signature(&super::unit_expression(&unit).expect("guarded"));
				let (quantity, given) = self.infer(*quantity)?;
				if given != target {
					return Err(Stop::Error(format!("DimensionError: {} cannot be converted to {}", shown(&given), shown(&target))));
				}
				Ok((quantity, given))
			}
			// `"distance " + d`: the quantity joins as its text, `"distance " + (d as km)` in km
			Node::Key(left, Op::Add, right) if super::joins_text(&left) || super::joins_text(&right) => {
				let (left_units, right_units) = (conversion_target(&left), conversion_target(&right));
				let (left, left_signature) = self.infer(*left)?;
				let (right, right_signature) = self.infer(*right)?;
				let left = self.as_text(left, &left_signature, left_units, AMOUNT_TEXT)?;
				let right = self.as_text(right, &right_signature, right_units, AMOUNT_TEXT)?;
				Ok((Node::Key(Box::new(left), Op::Add, Box::new(right)), vec![]))
			}
			// `r.distance == ø` of an optional unit field: ø has no dimension, any value may be it
			Node::Key(left, op @ (Op::Eq | Op::Ne), right) if is_nothing(&left) || is_nothing(&right) => {
				let (left, _) = self.infer(*left)?;
				let (right, _) = self.infer(*right)?;
				Ok((Node::Key(Box::new(left), op, Box::new(right)), vec![]))
			}
			// `2 m * "a"`: a text repeats a plain count of times (card units-text-repeat)
			Node::Key(left, Op::Mul, right) if is_text(&left) || is_text(&right) => {
				let (left, left_signature) = self.infer(*left)?;
				let (right, right_signature) = self.infer(*right)?;
				let count = combined(&left_signature, &right_signature, 1);
				match count.is_empty() {
					true => Ok((Node::Key(Box::new(left), Op::Mul, Box::new(right)), vec![])),
					false => Err(Stop::Error(format!("DimensionError: a text repeats a plain number of times, is given {}", shown(&count)))),
				}
			}
			Node::Key(left, op, right) if matches!(op, Op::Add | Op::Sub | Op::Mul | Op::Div) || op.is_comparison() => {
				let (left, left_signature) = self.infer(*left)?;
				let (right, right_signature) = self.infer(*right)?;
				let signature = self.arithmetic(op, &left_signature, &right_signature)?;
				Ok((folded(left, op, right), signature))
			}
			Node::Key(base, op @ (Op::Square | Op::Cube), nothing) if matches!(nothing.drop_meta(), Node::Empty) => {
				let (base, signature) = self.infer(*base)?;
				let exponent = if op == Op::Square { 2 } else { 3 };
				let signature = signature.iter().map(|(dimension, power)| (*dimension, power * exponent)).collect();
				Ok((Node::Key(Box::new(base), op, nothing), signature))
			}
			// `if c then a else b`: both branches have one signature
			Node::Key(if_then, Op::Else, otherwise) if matches!(if_then.drop_meta(), Node::Key(_, Op::Then, _)) => {
				let (if_then, then_signature) = self.infer(*if_then)?;
				let (otherwise, else_signature) = self.infer(*otherwise)?;
				if then_signature != else_signature {
					return Err(Stop::Error(format!("DimensionError: the branches give {} and {}", shown(&then_signature), shown(&else_signature))));
				}
				Ok((Node::Key(Box::new(if_then), Op::Else, Box::new(otherwise)), then_signature))
			}
			Node::Key(left, op @ (Op::Then | Op::If | Op::Do | Op::While | Op::Not | Op::And | Op::Or | Op::Range | Op::To), right) => {
				let (left, _) = self.infer(*left)?;
				let (right, signature) = self.infer(*right)?;
				let signature = if matches!(op, Op::Then | Op::Else) { signature } else { vec![] };
				Ok((Node::Key(Box::new(left), op, Box::new(right)), signature))
			}
			// `xs#2`: an element of a list of quantities
			Node::Key(list, Op::Hash, index) if matches!(list.drop_meta(), Node::Symbol(name) if self.lists.contains_key(name)) => {
				let element = self.lists[&list.name()].clone();
				let (index, _) = self.infer(*index)?;
				Ok((Node::Key(list, Op::Hash, Box::new(index)), element))
			}
			// `√a`: the powers halve, an odd power has no root
			Node::Key(nothing, Op::Sqrt, value) if matches!(nothing.drop_meta(), Node::Empty) => {
				let (value, signature) = self.infer(*value)?;
				if signature.iter().any(|(_, power)| power % 2 != 0) {
					return Err(Stop::Error(format!("DimensionError: √ of {} has no unit (an odd power)", shown(&signature))));
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
			// `q.s`: a field name is no unit
			Node::Key(object, Op::Dot, field) => {
				if let Some(unseen) = self.unseen_unit_field(&object, &field) {
					return Err(unseen);
				}
				let (object, signature) = self.infer(*object)?;
				if !signature.is_empty() {
					return Err(Stop::Unsupported);
				}
				Ok((Node::Key(Box::new(object), Op::Dot, field), vec![]))
			}
			Node::Key(left, op, right) => self.plain_key(*left, op, *right),
			Node::List(items, bracket, separator) => self.infer_list(items, bracket, separator),
			other => Ok((other, vec![])),
		}
	}

	/// Any other key: no quantity may take part (stage 1 does not know what it does with the unit)
	fn plain_key(&mut self, left: Node, op: Op, right: Node) -> Result<(Node, Signature), Stop> {
		let (left, left_signature) = self.infer(left)?;
		let (right, right_signature) = self.infer(right)?;
		if !left_signature.is_empty() || !right_signature.is_empty() {
			return Err(Stop::Unsupported);
		}
		Ok((Node::Key(Box::new(left), op, Box::new(right)), vec![]))
	}

	/// `x = q`, `x:any = q` (keeps the signature), `x:km = q` (checks it: annotated code is strict, P203), `p.dist = q`
	fn assignment(&mut self, target: Node, op: Op, value: Node) -> Result<(Node, Signature), Stop> {
		match target.drop_meta() {
			Node::Symbol(name) => self.assign_variable(name.clone(), target, op, value, None),
			Node::Key(name, Op::Colon, declared) if matches!(name.drop_meta(), Node::Symbol(_)) => match super::unit_expression(declared) {
				// the unit is no type the emitter knows: the variable is a plain number with this signature
				Some(units) => self.assign_variable(name.name(), name.as_ref().clone(), op, value, Some(signature(&units))),
				// `x: int = 1 m`: a number type holds plain numbers only (card units-annotation)
				None if is_number_type(declared) => self.assign_variable(name.name(), target.clone(), op, value, Some(vec![])),
				None => {
					self.declared_instance(&name.name(), declared);
					match self.instances.contains_key(&name.name()) {
						true => self.infer(value).map(|(value, _)| (Node::Key(Box::new(target.clone()), op, Box::new(value)), vec![])),
						false => self.assign_variable(name.name(), target.clone(), op, value, None),
					}
				}
			},
			Node::Key(object, Op::Dot, field) if self.field_signature(object, field).is_some() => {
				let held = self.field_signature(object, field).expect("guarded");
				let (value, given) = self.infer(value)?;
				if given != held {
					return Err(Stop::Error(format!("DimensionError: {} holds {}, is given {}", target.serialize().trim(), shown(&held), shown(&given))));
				}
				Ok((Node::Key(Box::new(target), op, Box::new(value)), held))
			}
			_ => self.plain_key(target, op, value),
		}
	}

	fn assign_variable(&mut self, name: String, target: Node, op: Op, value: Node, declared: Option<Signature>) -> Result<(Node, Signature), Stop> {
		if let Some(instance) = self.assigned_instance(&name, &target, op, &value) {
			return instance;
		}
		if let Some(mapped) = self.mapped_list(&value) {
			let (mapped, element) = mapped?;
			if !element.is_empty() {
				self.lists.insert(name, element);
			}
			return Ok((Node::Key(Box::new(target), op, Box::new(mapped)), vec![]));
		}
		if let Node::List(items, Bracket::Square, separator) = value.drop_meta() {
			let (items, element) = self.list_elements(items.clone())?;
			if !element.is_empty() {
				self.lists.insert(name, element);
				return Ok((Node::Key(Box::new(target), op, Box::new(Node::List(items, Bracket::Square, separator.clone()))), vec![]));
			}
		}
		// `q = p`, `ys = xs`: an alias has the signatures of what it names
		if let Node::Symbol(other) = value.drop_meta() {
			if let Some(fields) = self.objects.get(other).cloned() {
				self.objects.insert(name, fields);
				return Ok((Node::Key(Box::new(target), op, Box::new(value)), vec![]));
			}
			if let Some(element) = self.lists.get(other).cloned() {
				self.lists.insert(name, element);
				return Ok((Node::Key(Box::new(target), op, Box::new(value)), vec![]));
			}
		}
		if let Some(object) = self.object_literal(&value) {
			let (object, fields) = object?;
			self.objects.insert(name, fields);
			return Ok((Node::Key(Box::new(target), op, Box::new(object)), vec![]));
		}
		let (value, signature) = self.infer(value)?;
		if let Some(declared) = declared.filter(|declared| *declared != signature) {
			return Err(Stop::Error(format!("DimensionError: {name} is declared {}, is given {}", shown(&declared), shown(&signature))));
		}
		if let Some(earlier) = self.variables.get(&name).filter(|earlier| **earlier != signature) {
			return Err(Stop::Error(format!("DimensionError: {name} was {}, is given {}", shown(earlier), shown(&signature))));
		}
		self.variables.insert(name, signature.clone());
		Ok((Node::Key(Box::new(target), op, Box::new(value)), signature))
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
		let Node::List(_, bracket, separator) = value.drop_meta() else { return None };
		Some(Ok((Node::List(lowered, bracket.clone(), separator.clone()), fields)))
	}

	/// `p.dist` of an object with quantity fields, or of an instance of a class with unit fields: the field's signature
	/// (none for a plain field)
	fn field_signature(&self, object: &Node, field: &Node) -> Option<Signature> {
		let Node::Symbol(field) = field.drop_meta() else { return None };
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
			if let Some(element) = self.lists.get(list).cloned().filter(|_| ELEMENT_METHODS.contains(&method)) {
				let (argument, given) = self.infer(argument.clone())?;
				if given != element {
					return Err(Stop::Error(format!("DimensionError: {list} holds {}, is given {}", shown(&element), shown(&given))));
				}
				let call = Node::List(vec![Node::Symbol(method.to_string()), argument], Bracket::Round, Separator::None);
				return Ok((Node::Key(Box::new(object), Op::Dot, Box::new(call)), vec![]));
			}
		}
		let is_text = TEXT_WORDS.contains(&method) && arguments.is_empty();
		let (object, signature) = self.infer(object)?;
		if is_text && !signature.is_empty() {
			return Ok((self.as_text(object, &signature, None, AMOUNT_TEXT)?, vec![]));
		}
		let (call, _) = self.infer(call)?;
		if !signature.is_empty() {
			return Err(Stop::Unsupported);
		}
		Ok((Node::Key(Box::new(object), Op::Dot, Box::new(call)), vec![]))
	}

	fn arithmetic(&self, op: Op, left: &Signature, right: &Signature) -> Result<Signature, Stop> {
		match op {
			Op::Mul => Ok(combined(left, right, 1)),
			Op::Div => Ok(combined(left, right, -1)),
			_ if left != right => Err(dimension_error(op, left, right)),
			_ if op.is_comparison() => Ok(vec![]),
			_ => Ok(left.clone()),
		}
	}

	/// The source of a quantity's text at run time, `amount` an SI amount: `str(amount / (1/100)) + " cm"`
	fn quantity_source(&self, amount: &str, signature: &Signature, amount_text: &str) -> Result<String, Stop> {
		let units = display_factors(&self.display, signature);
		let per_unit = number_node(&super::scale(&units, |factor| base_unit(factor.unit.dimension))).ok_or(Stop::Unsupported)?;
		let amount_text = amount_text.replace(AMOUNT_PLACEHOLDER, &format!("({amount} / ({}))", per_unit.serialize().trim()));
		Ok(format!("{amount_text} + \"{}\"", super::unit_suffix(&units)))
	}

	/// A whole list of quantities as text, built at run time: `"[" + join(map(xs, x => str(x / 1/100) + " cm"), " ") + "]"`
	fn list_text(&self, name: &str, amount_text: &str) -> Result<Node, Stop> {
		let element = self.lists.get(name).ok_or(Stop::Unsupported)?;
		let source = format!("\"[\" + join(map({name}, item => {}), \" \") + \"]\"", self.quantity_source("item", element, amount_text)?);
		Ok(crate::warp_parser::parse(&source))
	}

	/// A whole object with quantity fields as text, built at run time: `{dist:500 m name:"run"}`. The quantity fields come
	/// first, the plain ones follow as an object of them shows them
	fn object_text(&self, name: &str, amount_text: &str) -> Result<Node, Stop> {
		let fields = self.objects.get(name).ok_or(Stop::Unsupported)?;
		let mut quantities = vec![];
		let mut plain = vec![];
		for (field, signature) in fields {
			match signature.is_empty() {
				true => plain.push(format!("{field}: {name}.{field}")),
				false => quantities.push(format!("\"{field}:\" + {}", self.quantity_source(&format!("{name}.{field}"), signature, amount_text)?)),
			}
		}
		let rest = match plain.is_empty() {
			true => "\"}\"".to_string(),
			false => format!("\" \" + text_form({{{}}})[1..]", plain.join(", ")),
		};
		Ok(crate::warp_parser::parse(&format!("\"{{\" + {} + {rest}", quantities.join(" + \" \" + "))))
	}

	/// The text of a whole list or object with quantities (`print xs`, `"${p}"`); None for any other value
	fn whole_text(&self, value: &Node, amount_text: &str) -> Option<Result<Node, Stop>> {
		match value.drop_meta() {
			Node::Symbol(name) if self.lists.contains_key(name) => Some(self.list_text(name, amount_text)),
			Node::Symbol(name) if self.objects.contains_key(name) => Some(self.object_text(name, amount_text)),
			_ => None,
		}
	}

	/// The last statement of the program when it is a whole list of quantities: its text
	fn final_list(&self, statement: &Node) -> Option<Result<Node, Stop>> {
		match statement.drop_meta() {
			Node::Symbol(name) if self.lists.contains_key(name) => Some(self.list_text(name, AMOUNT_TEXT)),
			_ => None,
		}
	}

	/// A value as it joins a text: a quantity as its text in `units` (else in the finest written units), anything else as it is
	fn as_text(&self, value: Node, signature: &Signature, units: Option<Vec<Factor>>, amount_text: &str) -> Result<Node, Stop> {
		if signature.is_empty() {
			return Ok(value);
		}
		let units = units.unwrap_or_else(|| display_factors(&self.display, signature));
		quantity_text(value, &units, amount_text).ok_or(Stop::Unsupported)
	}

	/// The items of a list literal and their one signature: items of different units are a DimensionError
	fn list_elements(&mut self, items: Vec<Node>) -> Result<(Vec<Node>, Signature), Stop> {
		let mut lowered = vec![];
		let mut element: Option<Signature> = None;
		for item in items {
			let (item, signature) = self.infer(item)?;
			match &element {
				Some(earlier) if *earlier != signature => {
					return Err(Stop::Error(format!("DimensionError: a list of {} and {}", shown(earlier), shown(&signature))));
				}
				_ => element = Some(signature),
			}
			lowered.push(item);
		}
		Ok((lowered, element.unwrap_or_default()))
	}

	/// The element signature of a list of quantities a word applies to: `sum(xs)` of a list variable, `max(a, b)` of quantities
	fn list_word(&mut self, word: &str, arguments: &[Node]) -> Option<Result<(Vec<Node>, Signature), Stop>> {
		if !ELEMENT_WORDS.contains(&word) && !COUNT_WORDS.contains(&word) {
			return None;
		}
		let (arguments, element) = match arguments {
			[list] => match (list.drop_meta(), self.mapped_list(list)) {
				(Node::Symbol(name), _) => (arguments.to_vec(), self.lists.get(name).cloned()?),
				(Node::List(items, Bracket::Square, separator), _) => match self.list_elements(items.clone()) {
					Ok((items, element)) if !element.is_empty() => (vec![Node::List(items, Bracket::Square, separator.clone())], element),
					Ok(_) => return None,
					Err(stop) => return Some(Err(stop)),
				},
				(_, Some(Ok((mapped, element)))) => (vec![mapped], element),
				(_, Some(Err(stop))) => return Some(Err(stop)),
				(_, None) => return None,
			},
			_ if ELEMENT_WORDS.contains(&word) => match self.list_elements(arguments.to_vec()) {
				Ok((items, element)) if !element.is_empty() => return Some(Ok((items, element))),
				Ok(_) => return None,
				Err(stop) => return Some(Err(stop)),
			},
			_ => return None,
		};
		let signature = if COUNT_WORDS.contains(&word) { vec![] } else { element };
		Some(Ok((arguments, signature)))
	}

	/// `runs.map(r => r.distance)` (or an `it` body) of a list of a class or of quantities: the list, its
	/// function's parameter an element, and the signature of the elements it maps to; None for any other value
	fn mapped_list(&mut self, list: &Node) -> Option<Result<(Node, Signature), Stop>> {
		let Node::Key(source, Op::Dot, call) = list.drop_meta() else { return None };
		let Node::List(items, Bracket::Round, separator) = call.drop_meta() else { return None };
		let [method, function] = items.as_slice() else { return None };
		// a table's list reads as `runs·load()`
		let source_name = match source.drop_meta() {
			Node::Symbol(name) => name.clone(),
			loaded => crate::database_tables::loaded_table(loaded)?,
		};
		if method.drop_meta().name() != MAP_WORD {
			return None;
		}
		let (parameter, body) = match function.drop_meta() {
			Node::Key(parameter, Op::FatArrow, body) => (parameter.drop_meta().name(), body.as_ref()),
			body => (crate::lambdas::IMPLICIT_PARAMETER.to_string(), body),
		};
		if let Some(class) = self.class_lists.get(&source_name).cloned() {
			self.declare_instance(parameter, class);
		} else if let Some(element) = self.lists.get(&source_name).cloned() {
			self.variables.insert(parameter, element);
		} else {
			return None;
		}
		Some(self.infer(body.clone()).map(|(body, element)| {
			let function = match function.drop_meta() {
				Node::Key(parameter, Op::FatArrow, _) => Node::Key(parameter.clone(), Op::FatArrow, Box::new(body)),
				_ => body,
			};
			let call = Node::List(vec![method.clone(), function], Bracket::Round, separator.clone());
			(Node::Key(source.clone(), Op::Dot, Box::new(call)), element)
		}))
	}

	/// `print q`, `q in km`, `return q`: the forms of stage 3 written as lists; None for any other list
	fn output_form(&mut self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Result<(Node, Signature), Stop>> {
		// a one-line block `{ print r.distance }` is its words too
		let call_or_words = matches!((bracket, separator), (Bracket::None | Bracket::Curly, Separator::Space) | (Bracket::Round, Separator::None));
		let word = |node: &Node, wanted: &str| matches!(node.drop_meta(), Node::Symbol(w) if w == wanted);
		if call_or_words {
			if let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) {
				if let Some(found) = self.list_word(name, &items[1..]) {
					return Some(found.map(|(arguments, signature)| (Node::List([vec![items[0].clone()], arguments].concat(), bracket.clone(), separator.clone()), signature)));
				}
			}
		}
		// `for x in xs` of a list of quantities: x has the element signature
		if let [for_word, variable, in_word, list, body] = items {
			if word(for_word, "for") && word(in_word, super::IN_WORD) {
				if let (Node::Symbol(name), Node::Symbol(list_name)) = (variable.drop_meta(), list.drop_meta()) {
					if let Some(element) = self.lists.get(list_name).cloned() {
						self.variables.insert(name.clone(), element);
						return Some(self.infer(body.clone()).map(|(body, _)| {
							(Node::List(vec![for_word.clone(), variable.clone(), in_word.clone(), list.clone(), body], bracket.clone(), separator.clone()), vec![])
						}));
					}
				}
			}
		}
		match items {
			[head, value] if call_or_words && TEXT_WORDS.iter().any(|text| word(head, text)) && self.whole_text(value, AMOUNT_TEXT).is_some() => {
				self.whole_text(value, AMOUNT_TEXT).map(|text| text.map(|text| (text, vec![])))
			}
			[head, value] if call_or_words && TEXT_WORDS.iter().any(|text| word(head, text)) => Some((|| {
				let (value, signature) = self.infer(value.clone())?;
				if signature.is_empty() {
					return Ok((Node::List(vec![head.clone(), value], bracket.clone(), separator.clone()), vec![]));
				}
				Ok((self.as_text(value, &signature, None, AMOUNT_TEXT)?, vec![]))
			})()),
			[head, value] if call_or_words && word(head, PRINT_WORD) && self.whole_text(value, PRINTED_AMOUNT_TEXT).is_some() => {
				self.whole_text(value, PRINTED_AMOUNT_TEXT).map(|text| text.map(|text| (Node::List(vec![head.clone(), text], bracket.clone(), separator.clone()), vec![])))
			}
			[head, value] if call_or_words && word(head, PRINT_WORD) => Some((|| {
				let shown_units = conversion_target(value);
				let (value, signature) = self.infer(value.clone())?;
				let text = self.as_text(value, &signature, shown_units, PRINTED_AMOUNT_TEXT)?;
				Ok((Node::List(vec![head.clone(), text], bracket.clone(), separator.clone()), vec![]))
			})()),
			[head, value] if *bracket == Bracket::None && word(head, RETURN_WORD) => Some((|| {
				let (value, signature) = self.infer(value.clone())?;
				if let Some(returns) = self.returns.last_mut() {
					returns.push(signature.clone());
				}
				Ok((Node::List(vec![head.clone(), value], bracket.clone(), separator.clone()), signature))
			})()),
			[quantity, in_word, unit] if *bracket == Bracket::None && word(in_word, super::IN_WORD) && super::unit_expression(unit).is_some() => Some((|| {
				let target = signature(&super::unit_expression(unit).expect("guarded"));
				let (quantity, given) = self.infer(quantity.clone())?;
				if given != target {
					return Err(Stop::Error(format!("DimensionError: {} cannot be converted to {}", shown(&given), shown(&target))));
				}
				Ok((quantity, given))
			})()),
			_ => None,
		}
	}

	/// `f(a, b)` of a user function with quantities (or units in its body): the call of its specialisation
	fn specialised_call(&mut self, items: &[Node]) -> Option<Result<(Node, Signature), Stop>> {
		let (head, arguments) = items.split_first()?;
		let Node::Symbol(name) = head.drop_meta() else { return None };
		let definition = self.definitions.get(name)?.clone();
		if definition.parameters.len() != arguments.len() {
			return None;
		}
		let mut lowered = vec![];
		let mut signatures = vec![];
		for argument in arguments {
			match self.infer(argument.clone()) {
				Ok((argument, signature)) => {
					lowered.push(argument);
					signatures.push(signature);
				}
				Err(stop) => return Some(Err(stop)),
			}
		}
		if signatures.iter().all(Vec::is_empty) && !mentions_units(&definition.body) {
			return None;
		}
		Some(self.specialise(name, &definition, signatures).map(|(specialised, result)| {
			(Node::List([vec![Node::Symbol(specialised)], lowered].concat(), Bracket::Round, Separator::None), result)
		}))
	}

	/// The specialisation of a function for the unit signatures of its arguments: its name and result signature
	fn specialise(&mut self, name: &str, definition: &Definition, signatures: Vec<Signature>) -> Result<(String, Signature), Stop> {
		let key = (name.to_string(), signatures.clone());
		if let Some(done) = self.specialised.get(&key) {
			return Ok(done.clone());
		}
		if self.in_progress.contains(&key) {
			return Err(Stop::Unsupported); // recursion with quantities: a later stage
		}
		self.in_progress.push(key.clone());
		let outer = std::mem::replace(&mut self.variables, definition.parameters.iter().cloned().zip(signatures).collect());
		self.returns.push(vec![]);
		let inferred = self.infer(definition.body.clone());
		let returns = self.returns.pop().unwrap_or_default();
		self.variables = outer;
		self.in_progress.pop();
		let (body, result) = inferred?;
		// every `return` gives the result's units: the first one decides, the others and the last statement agree
		let result = returns.first().cloned().unwrap_or(result);
		if let Some(other) = returns.iter().find(|signature| **signature != result) {
			return Err(Stop::Error(format!("DimensionError: {name} returns {} and {}", shown(&result), shown(other))));
		}
		let specialised = format!("{name}{SPECIALISATION_SEPARATOR}{}", self.specialised.len());
		let parameters = definition.parameters.iter().map(|parameter| Node::Symbol(parameter.clone()));
		let head = Node::List(std::iter::once(Node::Symbol(specialised.clone())).chain(parameters).collect(), Bracket::Round, Separator::None);
		self.new_definitions.push(Node::Key(Box::new(head), Op::Define, Box::new(body)));
		self.specialised.insert(key, (specialised.clone(), result.clone()));
		Ok((specialised, result))
	}

	fn infer_list(&mut self, items: Vec<Node>, bracket: Bracket, separator: Separator) -> Result<(Node, Signature), Stop> {
		if let Some(output) = self.output_form(&items, &bracket, &separator) {
			return output;
		}
		// `(q as km)`: parentheses around one expression keep its signature
		if let ([inner], Bracket::Round) = (items.as_slice(), &bracket) {
			if matches!(inner.drop_meta(), Node::Key(..)) || conversion_target(inner).is_some() {
				let (inner, signature) = self.infer(inner.clone())?;
				return Ok((Node::List(vec![inner], bracket, separator), signature));
			}
		}
		// a written object `{dist:500m name:"run"}` (only `:` entries: `{x = 1 m}` is a block); as the program's value its
		// fields' units go into `warp.units`
		let list = Node::List(items.clone(), bracket.clone(), separator.clone());
		let is_written_object = object_entries(&list).is_some_and(|entries| entries.iter().all(|(_, op, _)| *op == Op::Colon));
		if let Some(object) = self.object_literal(&list).filter(|_| is_written_object) {
			let (object, fields) = object?;
			if self.at_top {
				self.result_fields = Some(fields.into_iter().filter(|(_, signature)| !signature.is_empty()).collect());
			}
			return Ok((object, vec![]));
		}
		if bracket == Bracket::Round {
			if let Some(call) = self.constructed(&items) {
				return call;
			}
		}
		if bracket == Bracket::Round && separator == Separator::None {
			if let Some(call) = self.specialised_call(&items) {
				return call;
			}
		}
		let statements = matches!(separator, Separator::Semicolon | Separator::Newline) || bracket == Bracket::Curly;
		let is_loop = matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if LOOP_WORDS.contains(&word.as_str()));
		if is_loop {
			self.loop_instance(&items);
		}
		// `5 km`: an amount and a unit
		// the amount is a value: a number, an expression or a variable, never a word like `use` in `use m` (the libm module)
		let is_amount = |item: &Node| match item.drop_meta() {
			Node::Symbol(name) => self.variables.contains_key(name),
			Node::Text(_) | Node::Char(_) => false,
			_ => true,
		};
		let is_amount_and_unit = items.len() == 2 && bracket == Bracket::None && separator == Separator::Space && is_amount(&items[0])
			&& matches!(items[1].drop_meta(), Node::Symbol(name) if unit_named(name).is_some() && !self.variables.contains_key(name));
		let is_host_call = bracket == Bracket::Round && matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == HOST_CALL);
		// `(r.distance)`, `(a + b)`: parentheses around one expression group it (a lone word `(f)` may be a call)
		let grouping = bracket == Bracket::Round && matches!(items.as_slice(), [single] if !matches!(single.drop_meta(), Node::Symbol(_)));
		let in_host_call = self.in_host_call || is_host_call;
		let outer_host_call = std::mem::replace(&mut self.in_host_call, in_host_call);
		let mut lowered = vec![];
		let mut signatures = vec![];
		let count = items.len();
		for (index, item) in items.into_iter().enumerate() {
			// the program's last statement may show a whole list of quantities
			if statements && index + 1 == count && self.at_top {
				// a final object: its fields' units go into the `units` entry
				let fields = self.objects.get(&item.name()).cloned().or_else(|| self.list_instance_fields(&item));
				if let Some(fields) = fields.filter(|_| matches!(item.drop_meta(), Node::Symbol(_))) {
					self.result_fields = Some(fields.iter().filter(|(_, signature)| !signature.is_empty()).cloned().collect());
					lowered.push(item);
					signatures.push(vec![]);
					continue;
				}
				if let Some(text) = self.final_list(&item) {
					lowered.push(text?);
					signatures.push(vec![]);
					continue;
				}
			}
			let outer = std::mem::replace(&mut self.at_top, false);
			let inferred = self.infer(item);
			self.at_top = outer;
			let (item, signature) = inferred?;
			lowered.push(item);
			signatures.push(signature);
		}
		if is_amount_and_unit {
			let signature = combined(&signatures[0], &signatures[1], 1);
			let product = Node::Key(Box::new(lowered[0].clone()), Op::Mul, Box::new(lowered[1].clone()));
			return Ok((product, signature));
		}
		let in_host_call = std::mem::replace(&mut self.in_host_call, outer_host_call);
		let signature = if statements || grouping { signatures.last().cloned().unwrap_or_default() } else if is_loop || in_host_call || signatures.iter().all(Vec::is_empty) {
			vec![]
		} else {
			return Err(Stop::Unsupported); // a call, print, a list literal or interpolation of a quantity
		};
		Ok((Node::List(lowered, bracket, separator), signature))
	}
}
