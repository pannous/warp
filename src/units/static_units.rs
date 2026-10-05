//! Static units, stage 1 (notes/units_runtime.md): a program with quantities that `units::answer` cannot evaluate at compile
//! time (loops, branches) runs with plain exact numbers. Each expression's unit signature (dimension → power) is inferred
//! here and checked (`+ - ==` of different signatures is a DimensionError at compile time), each unit literal becomes its
//! amount in SI base units (`5 km` → 5000, `5 cm` → 1/20), and the program's final value is converted back to the finest
//! written unit after the run (`RESULT_UNITS`). Stage 2: a function called with quantities (or whose body has unit
//! literals) is specialised per unit signature of its arguments, `speed·u0`. A quantity anywhere these stages do not
//! cover (print, lists, interpolation, recursion) leaves the program unchanged: the unit stays the loud undefined-variable error.

use super::{finest_units, signature, unit_named, units_text, Dimension, Factor, Quantity, Unit, UNITS};
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

type Signature = Vec<(Dimension, i32)>;

thread_local! {
	/// The units of the final value of the last lowered program, to convert its run-time amount back (pipeline::eval_program)
	static RESULT_UNITS: RefCell<Option<Vec<Factor>>> = const { RefCell::new(None) };
}

/// Why a program is no stage-1 program: `Unsupported` leaves it unchanged, `Error` is a compile error
enum Stop {
	Unsupported,
	Error(String),
}

/// The program with unit literals as SI amounts and the units of its final value; None when it uses no units or uses
/// quantities beyond stage 1; Err for a dimension error
pub fn lower(program: &Node) -> Option<Result<Node, Node>> {
	if !super::needs_quantities(program) || super::defines_unit_name(program) {
		return None;
	}
	let mut written = vec![];
	program.visit(&mut |node| if let Node::Symbol(name) = node { written.extend(unit_named(name)) });
	let display = finest_units(&written.iter().map(|unit| Factor { unit, power: 1 }).collect::<Vec<_>>());
	let mut definitions = HashMap::new();
	program.visit(&mut |node| {
		if let Some((name, definition)) = function_definition(node) {
			definitions.insert(name, definition);
		}
	});
	let mut inference = Inference { variables: HashMap::new(), definitions, specialised: HashMap::new(), in_progress: vec![], new_definitions: vec![] };
	match inference.infer(program.clone()) {
		Ok((node, result)) => {
			let node = if inference.new_definitions.is_empty() {
				node
			} else {
				Node::List([std::mem::take(&mut inference.new_definitions), vec![node]].concat(), Bracket::None, Separator::Semicolon)
			};
			let units: Vec<Factor> = result.iter().map(|(dimension, power)| Factor { unit: display_unit(&display, *dimension), power: *power }).collect();
			RESULT_UNITS.with(|cell| *cell.borrow_mut() = (!units.is_empty()).then_some(units));
			Some(Ok(node))
		}
		Err(Stop::Error(message)) => Some(Err(crate::node::error(&message))),
		Err(Stop::Unsupported) => None,
	}
}

/// The units the last lowered program's final value has, once (None for a plain number)
pub(crate) fn take_result_units() -> Option<Vec<Factor>> {
	RESULT_UNITS.with(|cell| cell.borrow_mut().take())
}

/// The units as written: `km/h`
pub(crate) fn units_shown(units: &[Factor]) -> String {
	units_text(units)
}

/// A run-time amount in SI base units as the quantity it is in `units`
pub(crate) fn quantity_of(value: Node, units: &[Factor]) -> Node {
	let Some(amount) = exact_amount(&value) else { return value };
	let in_si = super::scale(units, |factor| base_unit(factor.unit.dimension));
	match in_si.inverse() {
		Some(per_unit) => Node::data(Quantity { amount: amount.mul(&per_unit), factors: units.to_vec() }),
		None => value,
	}
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

fn shown(signature: &Signature) -> String {
	let factors: Vec<Factor> = signature.iter().map(|(dimension, power)| Factor { unit: base_unit(*dimension), power: *power }).collect();
	if factors.is_empty() { "a plain number".to_string() } else { units_text(&factors) }
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
		Node::List(items, _, _) => items.iter().any(mentions_units),
		_ => false,
	}
}

/// A specialisation: the function and the unit signatures of its arguments
type Specialisation = (String, Vec<Signature>);

struct Inference {
	variables: HashMap<String, Signature>,
	definitions: HashMap<String, Definition>,
	/// Specialised functions: their name and result signature
	specialised: HashMap<Specialisation, (String, Signature)>,
	in_progress: Vec<Specialisation>,
	new_definitions: Vec<Node>,
}

impl Inference {
	fn infer(&mut self, node: Node) -> Result<(Node, Signature), Stop> {
		if let Some((_, definition)) = function_definition(&node) {
			// a body with unit literals compiles only in its specialisations
			let kept = if mentions_units(&definition.body) { Node::Empty } else { node };
			return Ok((kept, vec![]));
		}
		match node {
			Node::Meta { node, data } => {
				let (node, signature) = self.infer(*node)?;
				Ok((Node::Meta { node: Box::new(node), data }, signature))
			}
			Node::Symbol(name) => match (self.variables.get(&name), unit_named(&name)) {
				(Some(signature), _) => Ok((Node::Symbol(name.clone()), signature.clone())),
				(None, Some(unit)) => Ok((si_literal(unit).ok_or(Stop::Unsupported)?, vec![(unit.dimension, 1)])),
				(None, None) => Ok((Node::Symbol(name), vec![])),
			},
			Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
				let Node::Symbol(name) = target.drop_meta() else { return self.plain_key(*target, op, *value) };
				let name = name.clone();
				let (value, signature) = self.infer(*value)?;
				if let Some(earlier) = self.variables.get(&name).filter(|earlier| **earlier != signature) {
					return Err(Stop::Error(format!("DimensionError: {name} was {}, is given {}", shown(earlier), shown(&signature))));
				}
				self.variables.insert(name, signature.clone());
				Ok((Node::Key(target, op, Box::new(value)), signature))
			}
			Node::Key(target, op, value) if op.is_compound_assign() => {
				let (target, held) = self.infer(*target)?;
				let (value, given) = self.infer(*value)?;
				let result = self.arithmetic(op.base_op(), &held, &given)?;
				if result != held {
					return Err(dimension_error(op, &held, &given));
				}
				Ok((Node::Key(Box::new(target), op, Box::new(value)), held))
			}
			Node::Key(left, op, right) if matches!(op, Op::Add | Op::Sub | Op::Mul | Op::Div) || op.is_comparison() => {
				let (left, left_signature) = self.infer(*left)?;
				let (right, right_signature) = self.infer(*right)?;
				let signature = self.arithmetic(op, &left_signature, &right_signature)?;
				Ok((Node::Key(Box::new(left), op, Box::new(right)), signature))
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
			// `q.s`: a field name is no unit
			Node::Key(object, Op::Dot, field) => {
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

	fn arithmetic(&self, op: Op, left: &Signature, right: &Signature) -> Result<Signature, Stop> {
		match op {
			Op::Mul => Ok(combined(left, right, 1)),
			Op::Div => Ok(combined(left, right, -1)),
			_ if left != right => Err(dimension_error(op, left, right)),
			_ if op.is_comparison() => Ok(vec![]),
			_ => Ok(left.clone()),
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
		let inferred = self.infer(definition.body.clone());
		self.variables = outer;
		self.in_progress.pop();
		let (body, result) = inferred?;
		let specialised = format!("{name}{SPECIALISATION_SEPARATOR}{}", self.specialised.len());
		let parameters = definition.parameters.iter().map(|parameter| Node::Symbol(parameter.clone()));
		let head = Node::List(std::iter::once(Node::Symbol(specialised.clone())).chain(parameters).collect(), Bracket::Round, Separator::None);
		self.new_definitions.push(Node::Key(Box::new(head), Op::Define, Box::new(body)));
		self.specialised.insert(key, (specialised.clone(), result.clone()));
		Ok((specialised, result))
	}

	fn infer_list(&mut self, items: Vec<Node>, bracket: Bracket, separator: Separator) -> Result<(Node, Signature), Stop> {
		if bracket == Bracket::Round && separator == Separator::None {
			if let Some(call) = self.specialised_call(&items) {
				return call;
			}
		}
		let statements = matches!(separator, Separator::Semicolon | Separator::Newline) || bracket == Bracket::Curly;
		let is_loop = matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if LOOP_WORDS.contains(&word.as_str()));
		// `5 km`: an amount and a unit
		let is_amount_and_unit = items.len() == 2 && bracket == Bracket::None && separator == Separator::Space
			&& matches!(items[1].drop_meta(), Node::Symbol(name) if unit_named(name).is_some() && !self.variables.contains_key(name));
		let mut lowered = vec![];
		let mut signatures = vec![];
		for item in items {
			let (item, signature) = self.infer(item)?;
			lowered.push(item);
			signatures.push(signature);
		}
		if is_amount_and_unit {
			let signature = combined(&signatures[0], &signatures[1], 1);
			let product = Node::Key(Box::new(lowered[0].clone()), Op::Mul, Box::new(lowered[1].clone()));
			return Ok((product, signature));
		}
		let signature = if statements { signatures.last().cloned().unwrap_or_default() } else if is_loop || signatures.iter().all(Vec::is_empty) {
			vec![]
		} else {
			return Err(Stop::Unsupported); // a call, print, a list literal or interpolation of a quantity
		};
		Ok((Node::List(lowered, bracket, separator), signature))
	}
}
