//! Exact real arithmetic at compile time (Footguns.md "Exact real numbers", extensions/reals.rs).
//! A constant program that uses π, ℯ, τ, ⅈ, √, ∛ or sin/cos/tan/ln/exp is evaluated here in the
//! exact normal form: `√2*√2 == 2`, `sin(π/6) == 1/2`, `π > 3.14` by interval arithmetic.
//! Programs with anything else (functions, loops, imports, runtime input) take the normal WASM path,
//! where exact reals are lowered to f64; a WASM GC representation of them is future work.
//! Ints and fractions without generators never come here: they keep the i64 / ratio fast path.

use crate::extensions::numbers::Number;
use crate::extensions::reals::{Exact, Generator, Monomial, Rational, Real};
use crate::compile_time::{answer_of, fail, Stop};
use crate::node::{error, Node, Separator};
use crate::operators::Op;
use crate::type_tests::{is_float_type_word, is_real_type_word, is_text_type_word};
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::cmp::Ordering;
use std::collections::HashMap;

pub const FUNCTIONS: [&str; 6] = ["sin", "cos", "tan", "ln", "exp", STANDARD_PART];
/// st(x): the standard part of a finite hyperreal (wiki/hyperreals.md)
const STANDARD_PART: &str = "st";

/// `nearest(x, rational, limit, within: ε)` of an exact x is computed here, exactly (Exact::nearest_fraction); of a
/// float at run time by lib/prelude.warp
const NEAREST: &str = "nearest";
const NEAREST_KIND: &str = "rational";
const NEAREST_WITHIN: &str = "within";
const NEAREST_LIMIT: &str = "limit";

/// Largest integer exponent computed exactly; beyond it the power is approximated
const MAX_EXACT_EXPONENT: i64 = 10_000;

#[derive(Clone, Debug)]
enum Value {
	Real(Real),
	/// explicit `as float`: IEEE from here on
	Float(f64),
	Bool(bool),
	/// a text joined with an exact value keeps its symbolic form: `"f" + √2` is "f√2"
	Text(String),
}

type Evaluated = Result<Value, Stop>;
type Scope = HashMap<String, Value>;

/// The value of a constant program that uses exact reals, None for any other program
pub fn answer(program: &Node) -> Option<Node> {
	if !mentions_generator(program) {
		return None;
	}
	answer_of(evaluate(program, &mut Scope::new()), Value::into_node)
}

fn mentions_generator(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(Number::Real(_)) => true,
		Node::Key(_, Op::Sqrt | Op::Cbrt, _) => true,
		Node::Key(left, _, right) => mentions_generator(left) || mentions_generator(right),
		Node::List(items, _, _) => items.first().is_some_and(|head| function_name(head).is_some()) || items.iter().any(mentions_generator),
		_ => false,
	}
}

fn function_name(node: &Node) -> Option<&'static str> {
	match node.drop_meta() {
		Node::Symbol(name) => FUNCTIONS.iter().find(|f| *f == name).copied(),
		_ => None,
	}
}

/// Programs that were not evaluated exactly see exact reals as their f64 value (as before), except their constant
/// expressions with a rational or true/false value
pub fn lower(node: Node) -> Node {
	let node = folded_exact(node);
	let real_variables = real_variables(&node);
	lower_reals(with_real_type_arguments(node, &real_variables))
}

/// In a program evaluated at run time, a constant expression of exact reals whose value is rational or a truth is that
/// value: `xs.add(√2 * √2 == 2)` adds true, `r = √2 * √2` is 2 (card exact-reals); an irrational one (or one beyond
/// i64) stays for lower_reals to make a float
fn folded_exact(node: Node) -> Node {
	let statement_like = matches!(node.drop_meta(), Node::List(_, _, Separator::Semicolon | Separator::Newline) | Node::Key(_, Op::Assign | Op::Define, _));
	if !statement_like && mentions_generator(&node) {
		match evaluate(&node, &mut Scope::new()) {
			Ok(Value::Bool(truth)) => return if truth { Node::True } else { Node::False },
			// beyond i64 the run-time path stays as it was (√1e40 is a float there, floor of it out of int range)
			Ok(Value::Real(real)) if rational(&real).is_some_and(|q| q.numerator.to_i64().is_some() && q.denominator.to_i64().is_some()) => return Value::Real(real).into_node(),
			// a fraction nearest asked for is exact whatever its size
			Ok(value @ Value::Real(_)) if is_nearest_call(&node) => return value.into_node(),
			_ => {}
		}
	}
	match node {
		Node::Key(left, op, right) => Node::Key(Box::new(folded_exact(*left)), op, Box::new(folded_exact(*right))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(folded_exact).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(folded_exact(*node)), data },
		other => other,
	}
}

/// Variables assigned once, to a value that mentions an exact real (`x=π`): their type is the type of that value
fn real_variables(program: &Node) -> HashMap<String, Node> {
	let mut assignments: HashMap<String, Vec<Node>> = HashMap::new();
	program.visit(&mut |node| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = node {
			if let Node::Symbol(name) = target.drop_meta() {
				assignments.entry(name.clone()).or_default().push(value.as_ref().clone());
			}
		}
	});
	assignments.into_iter()
		.filter_map(|(name, mut values)| (values.len() == 1 && mentions_real(&values[0])).then(|| (name, values.remove(0))))
		.collect()
}

/// `type(x)` and `is_type(x, spec)` of such a variable ask about its value, before lowering makes it a float
fn with_real_type_arguments(node: Node, real_variables: &HashMap<String, Node>) -> Node {
	if real_variables.is_empty() {
		return node;
	}
	match node {
		Node::List(mut items, bracket, separator) => {
			let asks_type = matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(head)) if head == "type" || head == crate::type_tests::IS_TYPE);
			let variable_value = match items.get(1).map(Node::drop_meta) {
				Some(Node::Symbol(name)) if asks_type => real_variables.get(name),
				_ => None,
			};
			if let Some(value) = variable_value {
				items[1] = value.clone();
			}
			Node::List(items.into_iter().map(|item| with_real_type_arguments(item, real_variables)).collect(), bracket, separator)
		}
		Node::Key(left, op, right) => Node::Key(Box::new(with_real_type_arguments(*left, real_variables)), op, Box::new(with_real_type_arguments(*right, real_variables))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_real_type_arguments(*node, real_variables)), data },
		other => other,
	}
}

fn lower_reals(node: Node) -> Node {
	match node {
		Node::Number(Number::Real(real)) => match real {
			Real::Exact(exact) if exact.has_imaginary() => error(&format!("{exact} is complex: ⅈ is only supported in constant expressions")),
			Real::Exact(exact) if exact.has_epsilon() => error(&format!("{exact} is a hyperreal: ε and ω are only supported in constant expressions")),
			_ => Node::Number(Number::Float(real.to_f64())),
		},
		Node::Key(left, op, right) => Node::Key(Box::new(lower_reals(*left)), op, Box::new(lower_reals(*right))),
		Node::List(items, _, _) if is_type_of_real(&items) => Node::Symbol(type_name_before_lowering(&items[1])),
		// `π is real`: answered before π becomes a float
		Node::List(items, _, _) if is_type_test_of_real(&items) => {
			let Node::Text(spec) = items[2].drop_meta() else { unreachable!("guarded") };
			if crate::type_tests::type_matches(&type_name_before_lowering(&items[1]), spec) { Node::True } else { Node::False }
		}
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lower_reals).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower_reals(*node)), data },
		other => other,
	}
}

/// `type(π)` asks for the type before lowering makes π a float: the type name is `real`, whatever the representation
fn is_type_of_real(items: &[Node]) -> bool {
	matches!(items, [head, argument] if matches!(head.drop_meta(), Node::Symbol(name) if name == "type") && known_before_lowering(argument))
}

fn is_type_test_of_real(items: &[Node]) -> bool {
	matches!(items, [head, argument, spec] if matches!(head.drop_meta(), Node::Symbol(name) if name == crate::type_tests::IS_TYPE)
		&& matches!(spec.drop_meta(), Node::Text(_)) && known_before_lowering(argument))
}

/// An argument whose type lowering would change: a written exact real, or a constant expression of roots (`√2` is real)
fn known_before_lowering(argument: &Node) -> bool {
	mentions_real(argument) || mentions_generator(argument) && evaluate(argument, &mut Scope::new()).is_ok()
}

/// The type of the written number, else of the exact value of a constant expression (`2 * π` is real, `π / π` int)
fn type_name_before_lowering(argument: &Node) -> String {
	let exact_value = || evaluate(argument, &mut Scope::new()).ok().map(Value::into_node);
	let word = crate::analyzer::literal_number_type_word(argument).or_else(|| exact_value().and_then(|value| crate::analyzer::literal_number_type_word(&value)));
	match word {
		Some(word) => word.to_string(),
		None => crate::analyzer::shown_list_type_name(argument, &crate::analyzer::Scope::new()),
	}
}

/// Does lowering change anything
pub fn mentions_real(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(Number::Real(_)) | Node::Key(_, Op::Cbrt, _) => true,
		Node::Key(left, _, right) => mentions_real(left) || mentions_real(right),
		Node::List(items, _, _) => items.iter().any(mentions_real),
		_ => false,
	}
}

fn is_empty(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Empty)
}

fn evaluate(node: &Node, scope: &mut Scope) -> Evaluated {
	match node.drop_meta() {
		Node::Number(number) => number_value(number),
		Node::True => Ok(Value::Bool(true)),
		Node::False => Ok(Value::Bool(false)),
		Node::Text(text) => Ok(Value::Text(text.clone())),
		Node::Char(character) => Ok(Value::Text(character.to_string())),
		Node::Symbol(name) => scope.get(name).cloned().ok_or(Stop::Unsupported),
		Node::Key(left, op, right) => key(left, *op, right, scope),
		Node::List(items, _, separator) => list(items, separator, scope),
		_ => Err(Stop::Unsupported),
	}
}

fn number_value(number: &Number) -> Evaluated {
	let exact = |q: Rational| Ok(Value::Real(Real::Exact(Exact::rational(q))));
	match number {
		Number::Int(n) => exact(Rational::integer(*n)),
		Number::BigInt(n) => exact(Rational::integer((*n).clone())),
		Number::Quotient(n, d) => exact(Rational::new(BigInt::from(*n), BigInt::from(*d))),
		Number::BigQuotient(q) => exact((*q).clone()),
		Number::Float(f) if Number::is_exact_decimal(*f) => {
			let (n, d) = crate::wasm_emitter::exact::decimal_fraction(*f);
			exact(Rational::new(n, d))
		}
		Number::Float(f) => Ok(Value::Real(Real::Approx(*f))),
		Number::Real(real) => Ok(Value::Real((*real).clone())),
		Number::Complex(..) | Number::Nan | Number::Inf | Number::NegInf => Err(Stop::Unsupported),
	}
}

fn list(items: &[Node], separator: &Separator, scope: &mut Scope) -> Evaluated {
	match items {
		[] => Err(Stop::Unsupported),
		[single] => evaluate(single, scope),
		[head, argument] if function_name(head).is_some() && !scope.contains_key(&head.name()) => {
			let argument = evaluate(argument, scope)?;
			call(function_name(head).unwrap_or_default(), argument)
		}
		[head, value, kind, options @ ..] if head.name() == NEAREST && kind.name() == NEAREST_KIND && !scope.contains_key(NEAREST) => {
			let value = evaluate(value, scope)?;
			nearest(value, options, scope)
		}
		// `real x = sqrt(2)` declares as `x: real = sqrt(2)` does
		[head, declaration] if matches!(head.drop_meta(), Node::Symbol(word) if is_real_type_word(word) && !scope.contains_key(word)) => match declaration.drop_meta() {
			Node::Key(target, Op::Assign | Op::Define, value) => declare_real(target, value, scope),
			_ => Err(Stop::Unsupported),
		},
		[head, argument] if is_text_type_word(&head.name()) && !scope.contains_key(&head.name()) => {
			convert(evaluate(argument, scope)?, &head.name())
		}
		_ if matches!(separator, Separator::Semicolon | Separator::Newline) => {
			let mut last = Err(Stop::Unsupported);
			for statement in items {
				last = Ok(evaluate(statement, scope)?);
			}
			last
		}
		_ => Err(Stop::Unsupported),
	}
}

fn key(left: &Node, op: Op, right: &Node, scope: &mut Scope) -> Evaluated {
	match op {
		Op::Assign | Op::Define => {
			match left.drop_meta() {
				Node::Symbol(name) => {
					let value = evaluate(right, scope)?;
					scope.insert(name.clone(), value.clone());
					Ok(value)
				}
				// `x: real = sqrt(2)` keeps the exact root (card typed-real); other declared types take the run-time path
				Node::Key(target, Op::Colon, declared) if is_real_type_word(&declared.name()) => declare_real(target, right, scope),
				_ => Err(Stop::Unsupported),
			}
		}
		Op::Sqrt | Op::Cbrt | Op::Neg | Op::Abs if is_empty(left) => unary(op, evaluate(right, scope)?),
		Op::Square | Op::Cube if is_empty(right) => {
			let exponent = if op == Op::Square { 2 } else { 3 };
			arithmetic(evaluate(left, scope)?, Op::Pow, Value::Real(Real::Exact(Exact::integer(exponent))))
		}
		Op::As => {
			let value = evaluate(left, scope)?;
			match right.drop_meta() {
				Node::Symbol(target) => convert(value, target.as_str()),
				_ => Err(Stop::Unsupported),
			}
		}
		Op::And | Op::Or => match (evaluate(left, scope)?, evaluate(right, scope)?) {
			(Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(if op == Op::And { a && b } else { a || b })),
			_ => Err(Stop::Unsupported),
		},
		_ if op.is_comparison() => compare(evaluate(left, scope)?, op, evaluate(right, scope)?),
		Op::Add => match (evaluate(left, scope)?, evaluate(right, scope)?) {
			(left @ Value::Text(_), right) | (left, right @ Value::Text(_)) => Ok(Value::Text(text_of(left)? + &text_of(right)?)),
			(left, right) => arithmetic(left, op, right),
		},
		Op::Sub | Op::Mul | Op::Div | Op::Pow => arithmetic(evaluate(left, scope)?, op, evaluate(right, scope)?),
		_ => Err(Stop::Unsupported),
	}
}

fn is_nearest_call(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|head| head.name() == NEAREST))
}

/// The fraction `nearest(x, rational, limit, within: ε)` names; a float x is left to lib/prelude.warp
fn nearest(value: Value, options: &[Node], scope: &mut Scope) -> Evaluated {
	let Value::Real(Real::Exact(exact)) = value else { return Err(Stop::Unsupported) };
	let (mut limit, mut within) = (None, None);
	for option in options {
		match option.drop_meta() {
			Node::Key(name, Op::Colon | Op::Assign, amount) if name.name() == NEAREST_WITHIN => within = Some(exact_rational(evaluate(amount, scope)?)?),
			Node::Key(name, Op::Colon | Op::Assign, amount) if name.name() == NEAREST_LIMIT => limit = Some(exact_rational(evaluate(amount, scope)?)?.numerator),
			positional => limit = Some(exact_rational(evaluate(positional, scope)?)?.numerator),
		}
	}
	let fraction = exact.nearest_fraction(limit.as_ref(), within.as_ref()).map_err(Stop::Error)?;
	Ok(Value::Real(Real::Exact(Exact::rational(fraction))))
}

/// The exact value of a number argument: a float as the decimal it was written as (1e-40 is 1/10^40)
fn exact_rational(value: Value) -> Result<Rational, Stop> {
	match value {
		Value::Real(Real::Exact(exact)) => exact.as_rational().ok_or(Stop::Unsupported),
		Value::Real(Real::Approx(f)) | Value::Float(f) => Rational::of_decimal(f).ok_or(Stop::Unsupported),
		_ => Err(Stop::Unsupported),
	}
}

/// `x as float` / `as double` / every float alias: the f64 nearest to the exact value; `as real` / `as exact` keep it
fn convert(value: Value, target: &str) -> Evaluated {
	match (value, target) {
		(value, text) if is_text_type_word(text) => Ok(Value::Text(text_of(value)?)),
		(Value::Real(real), float) if is_float_type_word(float) => Ok(Value::Float(finite_f64(&real)?)),
		(value @ Value::Real(_), real) if is_real_type_word(real) => Ok(value),
		(value @ Value::Float(_), float) if is_float_type_word(float) => Ok(value),
		_ => Err(Stop::Unsupported),
	}
}

/// A variable declared `real` (or `exact`) holds the exact real as it is
fn declare_real(target: &Node, value: &Node, scope: &mut Scope) -> Evaluated {
	let Node::Symbol(name) = target.drop_meta() else { return Err(Stop::Unsupported) };
	let value = evaluate(value, scope)?;
	if !matches!(value, Value::Real(_)) {
		return Err(Stop::Unsupported);
	}
	scope.insert(name.clone(), value.clone());
	Ok(value)
}

/// The text form of a value: an exact real symbolically (`√2`, `π/2`), as it prints
fn text_of(value: Value) -> Result<String, Stop> {
	match value {
		Value::Text(text) => Ok(text),
		Value::Bool(_) => Err(Stop::Unsupported),
		number => Ok(number.into_node().serialize()),
	}
}

fn real_operand(value: Value) -> Result<Real, Stop> {
	match value {
		Value::Real(real) => Ok(real),
		_ => Err(Stop::Unsupported),
	}
}

fn unary(op: Op, value: Value) -> Evaluated {
	if let Value::Float(f) = value {
		return Ok(Value::Float(match op {
			Op::Sqrt => f.sqrt(),
			Op::Cbrt => f.cbrt(),
			Op::Neg => -f,
			_ => f.abs(),
		}));
	}
	let real = real_operand(value)?;
	Ok(Value::Real(match op {
		Op::Sqrt => root(real, 2)?,
		Op::Cbrt => root(real, 3)?,
		Op::Neg => real.neg(),
		_ => absolute(real)?,
	}))
}

fn absolute(real: Real) -> Result<Real, Stop> {
	match real {
		Real::Exact(exact) => match exact.sign() {
			Ok(Ordering::Less) => Ok(Real::Exact(exact.neg())),
			Ok(_) => Ok(Real::Exact(exact)),
			Err(message) => fail(message),
		},
		Real::Approx(f) => Ok(Real::Approx(f.abs())),
	}
}

fn approximate(value: f64, what: impl FnOnce() -> String) -> Result<Real, Stop> {
	if value.is_nan() {
		fail(format!("{} is not a real number", what()))
	} else {
		Ok(Real::Approx(value))
	}
}

fn root(real: Real, index: u32) -> Result<Real, Stop> {
	let symbol = if index == 2 { "√" } else { "∛" };
	if let Real::Exact(exact) = &real {
		if let Some(root) = exact.root(index) {
			return Ok(Real::Exact(root));
		}
		if exact.has_epsilon() {
			return fail(format!("{symbol}({exact}) has no exact form: only a single term with a power of ε divisible by {index} has a root"));
		}
		if exact.has_imaginary() {
			return fail(format!("{symbol}({exact}) has no exact form and complex approximations are not supported"));
		}
	}
	let f = real.to_f64();
	approximate(if index == 2 { f.sqrt() } else { f.cbrt() }, || format!("{symbol}({real})"))
}

fn arithmetic(left: Value, op: Op, right: Value) -> Evaluated {
	match (left, right) {
		(Value::Float(a), b) => float_arithmetic(a, op, float_of(b)?),
		(a, Value::Float(b)) => float_arithmetic(float_of(a)?, op, b),
		(Value::Real(a), Value::Real(b)) => Ok(Value::Real(real_arithmetic(a, op, b)?)),
		_ => Err(Stop::Unsupported), // booleans are not numbers
	}
}

/// The f64 of a real; a hyperreal has none (an infinitesimal is no float), which is an error, never 0
fn finite_f64(real: &Real) -> Result<f64, Stop> {
	match real {
		Real::Exact(exact) if exact.has_epsilon() => fail(format!("{exact} is a hyperreal and has no float value; st({exact}) is its standard part")),
		_ => Ok(real.to_f64()),
	}
}

fn float_of(value: Value) -> Result<f64, Stop> {
	match value {
		Value::Float(f) => Ok(f),
		Value::Real(real) => finite_f64(&real),
		Value::Bool(_) | Value::Text(_) => Err(Stop::Unsupported),
	}
}

fn float_arithmetic(a: f64, op: Op, b: f64) -> Evaluated {
	Ok(Value::Float(match op {
		Op::Add => a + b,
		Op::Sub => a - b,
		Op::Mul => a * b,
		Op::Div => a / b,
		_ => a.powf(b),
	}))
}

fn real_arithmetic(a: Real, op: Op, b: Real) -> Result<Real, Stop> {
	let (Real::Exact(x), Real::Exact(y)) = (&a, &b) else {
		if matches!(&a, Real::Exact(x) if x.has_imaginary()) || matches!(&b, Real::Exact(y) if y.has_imaginary()) {
			return fail(format!("{a} {op} {b}: complex approximations are not supported"));
		}
		let (f, g) = (a.to_f64(), b.to_f64());
		let result = match op {
			Op::Add => f + g,
			Op::Sub => f - g,
			Op::Mul => f * g,
			Op::Div if g == 0.0 => return fail(format!("division by zero: {a}/{b}")),
			Op::Div => f / g,
			_ => return power(a, b),
		};
		return approximate(result, || format!("{a} {op} {b}"));
	};
	match op {
		Op::Add => Ok(Real::Exact(x.add(y))),
		Op::Sub => Ok(Real::Exact(x.sub(y))),
		Op::Mul => Ok(Real::Exact(x.mul(y))),
		Op::Div if y.is_zero() => fail(format!("division by zero: {x}/0")),
		Op::Div => match y.inverse() {
			Some(inverse) => Ok(Real::Exact(x.mul(&inverse))),
			None if x.has_imaginary() || y.has_imaginary() => fail(format!("{x}/({y}) has no exact form yet")),
			// 1/(1+ε) = 1-ε+ε²-…: an exact Laurent polynomial cannot hold infinitely many terms
			None if x.has_epsilon() || y.has_epsilon() => fail(format!("{x}/({y}) needs infinitely many powers of ε, it has no exact form")),
			// no rationalization of sums yet: 1/(1+√2) is approximated
			None => approximate(x.to_f64() / y.to_f64(), || format!("{x}/({y})")),
		},
		_ => power(a, b),
	}
}

/// q when the value is an exact rational
fn rational(real: &Real) -> Option<Rational> {
	match real {
		Real::Exact(exact) => exact.as_rational(),
		Real::Approx(_) => None,
	}
}

/// n when the value is exactly ℯ^n
fn euler_power(real: &Real) -> Option<i64> {
	let Real::Exact(exact) = real else { return None };
	let (monomial, coefficient) = exact.0.iter().next().filter(|_| exact.0.len() == 1)?;
	match monomial.0.as_slice() {
		[(Generator::Euler, n)] if *coefficient == Rational::integer(1) => Some(*n),
		_ => None,
	}
}

fn power(base: Real, exponent: Real) -> Result<Real, Stop> {
	if let Some(n) = euler_power(&base) {
		let scaled = real_arithmetic(Real::Exact(Exact::integer(n)), Op::Mul, exponent)?;
		return exp(scaled);
	}
	if let (Real::Exact(x), Some(q)) = (&base, rational(&exponent)) {
		let small = |n: &BigInt| n.to_i64().filter(|n| n.abs() <= MAX_EXACT_EXPONENT);
		if let (Some(numerator), Some(denominator)) = (small(&q.numerator), small(&q.denominator)) {
			if x.is_zero() && numerator < 0 {
				return fail("division by zero: 0 to a negative power");
			}
			if let Some(raised) = x.pow(numerator) {
				match denominator {
					1 => return Ok(Real::Exact(raised)),
					2 | 3 => {
						if let Some(root) = raised.root(denominator as u32) {
							return Ok(Real::Exact(root));
						}
					}
					_ => {}
				}
			}
		}
	}
	if matches!(&base, Real::Exact(x) if x.has_imaginary()) || matches!(&exponent, Real::Exact(y) if y.has_imaginary()) {
		return fail(format!("({base})^({exponent}) has no exact form and complex approximations are not supported"));
	}
	approximate(base.to_f64().powf(exponent.to_f64()), || format!("({base})^({exponent})"))
}

fn call(function: &str, argument: Value) -> Evaluated {
	if let Value::Float(f) = argument {
		return Ok(Value::Float(match function {
			"sin" => f.sin(),
			"cos" => f.cos(),
			"tan" => f.tan(),
			"ln" => f.ln(),
			STANDARD_PART => f,
			_ => f.exp(),
		}));
	}
	let real = real_operand(argument)?;
	if let Real::Exact(exact) = &real {
		if function == STANDARD_PART {
			return exact.standard_part().map(|part| Value::Real(Real::Exact(part))).map_err(Stop::Error);
		}
		if exact.has_epsilon() {
			return fail(format!("{function}({exact}) of a hyperreal has no exact form"));
		}
	}
	if function == STANDARD_PART {
		return Ok(Value::Real(real)); // a real is its own standard part
	}
	Ok(Value::Real(match function {
		"sin" | "cos" | "tan" => trigonometric(function, real)?,
		"ln" => logarithm(real)?,
		_ => exp(real)?,
	}))
}

/// sin/cos/tan at multiples of π/12 are exact, anything else is approximated
fn trigonometric(function: &str, real: Real) -> Result<Real, Stop> {
	if let Real::Exact(exact) = &real {
		if let Some(q) = exact.pi_multiple() {
			let twelfths = q.mul(&Rational::integer(12));
			if twelfths.is_integer() {
				let k = (twelfths.numerator % BigInt::from(24) + BigInt::from(24)) % BigInt::from(24);
				let k = k.to_i64().unwrap_or(0);
				return match function {
					"sin" => Ok(Real::Exact(sine_table(k))),
					"cos" => Ok(Real::Exact(sine_table((k + 6) % 24))),
					_ => tangent_table(k % 12).map(Real::Exact).ok_or_else(|| Stop::Error(format!("tan({exact}) is undefined"))),
				};
			}
		}
		if exact.has_imaginary() {
			return fail(format!("{function}({exact}) of a complex number is not supported"));
		}
	}
	let f = real.to_f64();
	approximate(
		match function {
			"sin" => f.sin(),
			"cos" => f.cos(),
			_ => f.tan(),
		},
		|| format!("{function}({real})"),
	)
}

fn square_root(n: i64) -> Exact {
	Exact::generator(Generator::SquareRoot(BigInt::from(n)))
}

fn fraction(n: i64, d: i64) -> Rational {
	Rational::new(BigInt::from(n), BigInt::from(d))
}

/// sin(k·π/12) for k in 0..24
fn sine_table(k: i64) -> Exact {
	if k >= 12 {
		return sine_table(k - 12).neg();
	}
	if k > 6 {
		return sine_table(12 - k);
	}
	let quarter = fraction(1, 4);
	match k {
		0 => Exact::integer(0),
		1 => square_root(6).sub(&square_root(2)).scale(&quarter),
		2 => Exact::rational(fraction(1, 2)),
		3 => square_root(2).scale(&fraction(1, 2)),
		4 => square_root(3).scale(&fraction(1, 2)),
		5 => square_root(6).add(&square_root(2)).scale(&quarter),
		_ => Exact::integer(1),
	}
}

/// tan(k·π/12) for k in 0..12, None at π/2
fn tangent_table(k: i64) -> Option<Exact> {
	if k > 6 {
		return tangent_table(12 - k).map(|t| t.neg());
	}
	Some(match k {
		0 => Exact::integer(0),
		1 => Exact::integer(2).sub(&square_root(3)),
		2 => square_root(3).scale(&fraction(1, 3)),
		3 => Exact::integer(1),
		4 => square_root(3),
		5 => Exact::integer(2).add(&square_root(3)),
		_ => return None,
	})
}

/// ln(1) = 0, ln(ℯ^n) = n, else approximated
fn logarithm(real: Real) -> Result<Real, Stop> {
	if let Some(n) = euler_power(&real) {
		return Ok(Real::Exact(Exact::integer(n)));
	}
	if rational(&real) == Some(Rational::integer(1)) {
		return Ok(Real::Exact(Exact::integer(0)));
	}
	let positive = match &real {
		Real::Exact(exact) if exact.has_imaginary() => return fail(format!("ln({exact}) of a complex number is not supported")),
		Real::Exact(exact) => exact.sign().map_err(Stop::Error)? == Ordering::Greater,
		Real::Approx(f) => *f > 0.0,
	};
	if !positive {
		return fail(format!("ln({real}) is undefined: not a positive number"));
	}
	approximate(real.to_f64().ln(), || format!("ln({real})"))
}

/// exp(n + qⅈπ) = ℯ^n · (cos qπ + ⅈ sin qπ) for integer n and q·12 integer, else approximated
fn exp(real: Real) -> Result<Real, Stop> {
	let Real::Exact(exact) = &real else {
		return approximate(real.to_f64().exp(), || format!("exp({real})"));
	};
	let rotation_monomial = Monomial(vec![(Generator::Pi, 1), (Generator::Imaginary, 1)]);
	let (mut integer, mut turn, mut rest) = (0i64, Rational::zero(), false);
	for (monomial, coefficient) in &exact.0 {
		if monomial.0.is_empty() && coefficient.is_integer() {
			match coefficient.numerator.to_i64() {
				Some(n) if n.abs() <= MAX_EXACT_EXPONENT => integer = n,
				_ => rest = true,
			}
		} else if *monomial == rotation_monomial {
			turn = coefficient.clone();
		} else {
			rest = true;
		}
	}
	if rest {
		if exact.has_imaginary() {
			return fail(format!("exp({exact}) has no exact form and complex approximations are not supported"));
		}
		return approximate(exact.to_f64().exp(), || format!("exp({exact})"));
	}
	let magnitude = Exact::euler().pow(integer).unwrap_or_else(|| Exact::integer(1));
	if turn.is_zero() {
		return Ok(Real::Exact(magnitude));
	}
	let angle = Real::Exact(Exact::pi().scale(&turn));
	let (Real::Exact(cosine), Real::Exact(sine)) = (trigonometric("cos", angle.clone())?, trigonometric("sin", angle)?) else {
		return fail(format!("exp({exact}) has no exact form and complex approximations are not supported"));
	};
	Ok(Real::Exact(magnitude.mul(&cosine.add(&sine.mul(&Exact::imaginary())))))
}

fn compare(left: Value, op: Op, right: Value) -> Evaluated {
	let ordering = match (left, right) {
		(Value::Bool(a), Value::Bool(b)) if op.is_equality() => return Ok(Value::Bool((a == b) == (op == Op::Eq))),
		(Value::Real(a), Value::Real(b)) if op.is_equality() => {
			let equal = a.equals(&b).map_err(Stop::Error)?;
			return Ok(Value::Bool(equal == (op == Op::Eq)));
		}
		(Value::Real(a), Value::Real(b)) => a.compare(&b).map_err(Stop::Error)?,
		(a, b) => match float_of(a)?.partial_cmp(&float_of(b)?) {
			Some(ordering) => ordering,
			None => return Ok(Value::Bool(op == Op::Ne)),
		},
	};
	Ok(Value::Bool(op.holds(ordering)))
}

impl Value {
	fn into_node(self) -> Node {
		match self {
			Value::Bool(truth) => if truth { Node::True } else { Node::False },
			Value::Float(f) => Node::Number(Number::Float(f)),
			Value::Real(Real::Exact(exact)) => match exact.as_rational() {
				Some(q) if q.is_integer() => Node::Number(Number::from_bigint(q.numerator)),
				Some(q) => Node::Number(Number::ratio(Number::from_bigint(q.numerator), Number::from_bigint(q.denominator))),
				None => Node::Number(Number::real(Real::Exact(exact))),
			},
			Value::Real(approximation) => Node::Number(Number::real(approximation)),
			Value::Text(text) => Node::Text(text),
		}
	}
}
