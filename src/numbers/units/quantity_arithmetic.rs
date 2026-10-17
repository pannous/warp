//! Units as written and their scales: unit expressions (`km/h`), conversions (`150 cm in m`), durations of the time module
use super::*;

/// Milliseconds in a day, a duration's `days` counted at their nominal length
const DAY_MILLISECONDS: i128 = 86_400_000;
const NANOS_PER_MILLISECOND: i128 = 1_000_000;

/// `q as cm` under a power: the quantity and the powered target units
pub(super) fn powered_conversion(conversion: &Node, op: Op) -> Option<(&Node, Vec<Factor>)> {
	let Node::Key(quantity, Op::As, unit) = conversion.drop_meta() else {
		return None;
	};
	Some((quantity, powered(unit_expression(unit)?, exponent_of(op))))
}

/// The exponent of `²` and `³`
pub(super) fn exponent_of(op: Op) -> i32 {
	if op == Op::Square {
		2
	} else {
		3
	}
}

pub(super) fn powered(factors: Vec<Factor>, exponent: i32) -> Vec<Factor> {
	factors.into_iter().map(|factor| Factor { power: factor.power * exponent, ..factor }).collect()
}

/// A conversion target written as units: `m`, `minutes`, `cm²`, `km/h`, `kg*m/s²`
pub(super) fn unit_expression(node: &Node) -> Option<Vec<Factor>> {
	factor_expression(node, &|leaf| target_unit(leaf).map(|unit| vec![Factor { unit, power: 1 }]))
}

/// Products, quotients and powers of what `leaf` reads: units (`km/h`) or dimensions (`length/duration`)
pub(super) fn factor_expression(node: &Node, leaf: &dyn Fn(&Node) -> Option<Vec<Factor>>) -> Option<Vec<Factor>> {
	match node.drop_meta() {
		Node::Key(base, op @ (Op::Square | Op::Cube), nothing) if nothing.is_empty_node() => {
			Some(powered(factor_expression(base, leaf)?, exponent_of(*op)))
		}
		Node::Key(left, Op::Mul, right) => {
			Some([factor_expression(left, leaf)?, factor_expression(right, leaf)?].concat())
		}
		Node::Key(left, Op::Div, right) => {
			Some([factor_expression(left, leaf)?, inverse(&factor_expression(right, leaf)?)].concat())
		}
		Node::List(items, Bracket::Round, _) if items.len() == 1 => factor_expression(&items[0], leaf),
		other => leaf(other),
	}
}

/// `q as km/h`, `q in km/h`: the quantity converted and the units it converts to
pub(crate) fn conversion(node: &Node) -> Option<(&Node, Vec<Factor>)> {
	match node.drop_meta() {
		Node::Key(quantity, Op::As, unit) => Some((quantity, unit_expression(unit)?)),
		Node::List(items, Bracket::None, Separator::Space) => match items.as_slice() {
			[quantity, word, unit] if matches!(word.drop_meta(), Node::Symbol(w) if w == IN_WORD) => {
				Some((quantity, unit_expression(unit)?))
			}
			_ => None,
		},
		_ => None,
	}
}

pub(super) fn inverse(factors: &[Factor]) -> Vec<Factor> {
	factors.iter().map(|factor| Factor { power: -factor.power, ..*factor }).collect()
}

/// The unit given to the last amount of an expression: `a == 3010` with `meters` is `a == (3010 meters)`
pub(super) fn with_unit(expression: &Node, unit: &Node) -> Node {
	match expression.drop_meta() {
		Node::Key(left, op, right) => Node::Key(left.clone(), *op, Box::new(with_unit(right, unit))),
		amount => Node::List(vec![amount.clone(), unit.clone()], Bracket::None, Separator::Space),
	}
}

/// `global x = 5m`: a main-level declaration functions write to
pub(crate) fn is_global_keyword(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if word == crate::words::GLOBAL_WORD)
}

/// `150 cm in m` is `3/2 m`, `6 m² in cm²` is `60000 cm²`, `36 km/h in m/s` is `10 m/s`; other dimensions: DimensionError
/// A duration of the time module in a time unit: `2 hours in minutes` is `120 min` (P36). None when the word names no
/// time unit; an error for months (no fixed length) or a part of a millisecond
pub fn duration_in(duration: &crate::time::Duration, unit_word: &str) -> Option<Node> {
	let target = target_unit(&Node::Symbol(unit_word.to_string()))?;
	if target.dimension != Dimension::duration() {
		return Some(error(&format!("DimensionError: {duration} cannot be converted to {}", target.name)));
	}
	let milliseconds = match duration_milliseconds(duration, target.name) {
		Ok(milliseconds) => milliseconds?,
		Err(message) => return Some(error(&message)),
	};
	let millisecond = unit_named("ms").expect("a unit");
	Some(Node::data(Quantity {
		amount: Rational::integer(milliseconds)
			.mul(&Rational::new(BigInt::from(millisecond.factor), BigInt::from(target.factor))),
		factors: vec![Factor { unit: target, power: 1 }],
	}))
}

/// One of a time unit as the time module's exact duration (decision duration-units): `h` is 1 hour; none for a unit of
/// another quantity or one that is no whole number of nanoseconds
pub(super) fn unit_duration(unit: &Unit) -> Option<crate::time::Duration> {
	let millisecond = unit_named("ms").expect("a unit");
	let steps = unit.factor as i128 * NANOS_PER_MILLISECOND;
	let exact = unit.dimension == Dimension::duration() && steps % millisecond.factor as i128 == 0;
	exact.then(|| crate::time::Duration { nanos: steps / millisecond.factor as i128, ..Default::default() })
}

/// A duration of the time module in whole milliseconds; an error for months (no fixed length in `target`) or a part of
/// a millisecond, None beyond the 64-bit integers
pub(super) fn duration_milliseconds(duration: &crate::time::Duration, target: &str) -> Result<Option<i64>, String> {
	if duration.months != 0 {
		return Err(format!("DimensionError: {duration} has no fixed length in {target}"));
	}
	let nanos = duration.nanos + duration.days as i128 * DAY_MILLISECONDS * NANOS_PER_MILLISECOND;
	if nanos % NANOS_PER_MILLISECOND != 0 {
		return Err(format!("{duration} is no whole number of milliseconds"));
	}
	Ok(i64::try_from(nanos / NANOS_PER_MILLISECOND).ok())
}

/// The amount of a quantity in other units, as a factor: Π (unit/target)^power over its factors (1 km/h in m/s is 5/18)
pub(super) fn scale(factors: &[Factor], target: impl Fn(&Factor) -> &'static Unit) -> Rational {
	factors.iter().fold(Rational::integer(1), |product, factor| {
		let ratio = Rational::new(BigInt::from(factor.unit.factor), BigInt::from(target(factor).factor));
		let ratio = if factor.power > 0 { ratio } else { ratio.inverse().expect("unit factors are positive") };
		(0..factor.power.unsigned_abs()).fold(product, |product, _| product.mul(&ratio))
	})
}

/// `1 m / 3 m` is the number 1/3
pub(super) fn quotient_node(amount: &Rational) -> Node {
	Node::Number(Number::from_rational(amount.clone()))
}

/// The integer nearest the amount, halves away from zero
pub(super) fn nearest(amount: &Rational) -> Option<i64> {
	let magnitude = (amount.numerator.magnitude() * 2u32 + amount.denominator.magnitude())
		/ (amount.denominator.magnitude() * 2u32);
	BigInt::from_biguint(amount.numerator.sign(), magnitude).to_i64()
}

/// The finest unit of each dimension among the factors: the unit a result counts in ("unit sums use the finer unit")
pub(super) fn finest_units(factors: &[Factor]) -> Vec<&'static Unit> {
	let mut finest: Vec<&'static Unit> = vec![];
	for factor in factors {
		match finest.iter_mut().find(|unit| unit.dimension == factor.unit.dimension) {
			Some(unit) if factor.unit.factor < unit.factor => *unit = factor.unit,
			Some(_) => {}
			None => finest.push(factor.unit),
		}
	}
	finest
}

/// Comparison of two quantities in their finer units: `3km == 3000m`, answered yes or no
/// `x = 1 m; x = 2 s`: a variable keeps its dimension, as the static units pass checks (card units-reassign)
/// `x = 1200; x = 1.2kHz`: a variable keeps its dimension; a plain number becomes a quantity by its first value's unit
pub(crate) fn reassigned_dimension(name: &str, was: &str, was_plain: bool, given: &str) -> String {
	let hint = if was_plain { ": give its first value a unit too" } else { "" };
	format!("DimensionError: {name} was {was}, is given {given}{hint}")
}
