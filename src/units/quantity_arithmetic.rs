//! Arithmetic on quantities: amounts times units, conversions (`150 cm in m`), scales and own zeros (°C), powers, comparisons in the finer unit, tolerances and ranges
use super::*;

/// `amount unit`: a decimal amount is exact (`0.3 s` is 3/10 s, card fractional-durations), any other computes as usual
pub(super) fn amount_times(amount: &Node, unit: Value, variables: &mut Variables) -> Evaluated {
	let decimal = match amount.drop_meta() {
		Node::Number(Number::Float(value)) => Rational::of_decimal(*value),
		_ => None,
	};
	match (decimal, unit) {
		(Some(exact), Value::Quantity(quantity)) => {
			Ok(Value::Quantity(quantity.with_amount(quantity.amount.mul(&exact))))
		}
		(_, unit) => arithmetic(evaluate_in(amount, variables)?, Op::Mul, unit),
	}
}

/// `(23/18)`, the exact fraction a quantity shows as its amount
pub(super) fn written_fraction(amount: &Node) -> Option<Rational> {
	let Node::List(items, Bracket::Round, _) = amount.drop_meta() else {
		return None;
	};
	let [Node::Key(numerator, Op::Div, denominator)] =
		items.as_slice().iter().map(Node::drop_meta).collect::<Vec<_>>()[..]
	else {
		return None;
	};
	match (numerator.drop_meta(), denominator.drop_meta()) {
		(Node::Number(Number::Int(n)), Node::Number(Number::Int(d))) if *d != 0 => {
			Some(Rational::new(BigInt::from(*n), BigInt::from(*d)))
		}
		_ => None,
	}
}

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
	factors
		.into_iter()
		.map(|factor| Factor {
			power: factor.power * exponent,
			..factor
		})
		.collect()
}

/// A conversion target written as units: `m`, `minutes`, `cm²`, `km/h`, `kg*m/s²`
pub(super) fn unit_expression(node: &Node) -> Option<Vec<Factor>> {
	match node.drop_meta() {
		Node::Key(base, op @ (Op::Square | Op::Cube), nothing) if matches!(nothing.drop_meta(), Node::Empty) => {
			Some(powered(unit_expression(base)?, exponent_of(*op)))
		}
		Node::Key(left, Op::Mul, right) => Some([unit_expression(left)?, unit_expression(right)?].concat()),
		Node::Key(left, Op::Div, right) => Some([unit_expression(left)?, inverse(&unit_expression(right)?)].concat()),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => unit_expression(&items[0]),
		other => target_unit(other).map(|unit| vec![Factor { unit, power: 1 }]),
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
	factors
		.iter()
		.map(|factor| Factor {
			power: -factor.power,
			..*factor
		})
		.collect()
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

pub(super) fn is_unit_word(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if unit_named(name).is_some())
}

pub(super) fn negate(value: Value) -> Evaluated {
	match value {
		Value::Number(n) => Ok(Value::Number(-n)),
		Value::Quantity(quantity) => Ok(Value::Quantity(quantity.with_amount(quantity.amount.neg()))),
		Value::Tolerance(_) | Value::Range(_) => fail("cannot negate a value with tolerance or a range"),
		Value::Bool(_) => Err(Stop::Unsupported),
	}
}

pub(super) fn arithmetic(left: Value, op: Op, right: Value) -> Evaluated {
	match (left, op, right) {
		(
			left @ (Value::Tolerance(_) | Value::Range(_)),
			Op::Eq | Op::Ne,
			right @ (Value::Tolerance(_) | Value::Range(_)),
		) => same_span(left, op, right),
		// `(5 ± 1) * 2` without units: an uncertain value, propagated at run time (wasm_emitter/uncertain.rs)
		(left, Op::Add | Op::Sub | Op::Mul | Op::Div, right)
			if [&left, &right].iter().all(|value| unitless_number(value))
				&& [&left, &right].iter().any(|value| matches!(value, Value::Tolerance(_))) =>
		{
			Err(Stop::Unsupported)
		}
		(Value::Tolerance(_) | Value::Range(_), _, _) | (_, _, Value::Tolerance(_) | Value::Range(_)) => {
			fail(format!("{TOLERANCE_ARITHMETIC}: {op}"))
		}
		(left, Op::PlusMinus, right) => tolerance(left, right),
		(Value::Number(from), Op::Sub, Value::Quantity(end)) if end.single_unit().is_some() => range(from, end),
		(Value::Number(a), Op::Add | Op::Sub | Op::Mul, Value::Number(b)) => {
			let result = match op {
				Op::Add => a.checked_add(b),
				Op::Sub => a.checked_sub(b),
				_ => a.checked_mul(b),
			};
			result.map(Value::Number).ok_or(Stop::Unsupported)
		}
		(Value::Quantity(a), Op::Add | Op::Sub, Value::Quantity(b)) => sum(a, op, b),
		(Value::Quantity(a), Op::Eq | Op::Ne | Op::Lt | Op::Gt | Op::Le | Op::Ge, Value::Quantity(b)) => {
			compare(a, op, b)
		}
		(Value::Quantity(a), Op::Mul, Value::Quantity(b)) => {
			combine(a.amount.mul(&b.amount), [a.factors.clone(), b.factors.clone()].concat())
		}
		(Value::Quantity(a), Op::Div, Value::Quantity(b)) if !b.amount.is_zero() => combine(
			a.amount.mul(&b.amount.inverse().expect("not zero")),
			[a.factors.clone(), inverse(&b.factors)].concat(),
		),
		(Value::Quantity(_), Op::Div, Value::Quantity(_)) => fail("division by zero"),
		(Value::Quantity(q), Op::Pow, Value::Number(n)) => power(q, n),
		(Value::Number(_), Op::Add, Value::Quantity(other))
		| (Value::Quantity(other), Op::Add | Op::Sub, Value::Number(_)) => fail(format!(
			"incompatible operands: a plain number and {}",
			units_text(&other.factors)
		)),
		(Value::Number(n), Op::Mul, Value::Quantity(q)) | (Value::Quantity(q), Op::Mul, Value::Number(n)) => {
			Ok(Value::Quantity(q.with_amount(q.amount.mul(&Rational::integer(n)))))
		}
		(Value::Quantity(q), Op::Div, Value::Number(n)) if n != 0 => Ok(Value::Quantity(
			q.with_amount(q.amount.mul(&Rational::new(BigInt::from(1), BigInt::from(n)))),
		)),
		_ => Err(Stop::Unsupported),
	}
}

/// `150 cm in m` is `3/2 m`, `6 m² in cm²` is `60000 cm²`, `36 km/h in m/s` is `10 m/s`; other dimensions: DimensionError
pub(super) fn convert(value: Value, target: Vec<Factor>) -> Evaluated {
	let Value::Quantity(quantity) = value else {
		return Err(Stop::Unsupported);
	};
	if signature(&quantity.factors) != signature(&target) {
		return fail(format!(
			"DimensionError: {quantity} cannot be converted to {}",
			units_text(&target)
		));
	}
	if let (Some(from), [Factor { unit: to, power: 1 }]) = (quantity.single_unit(), target.as_slice()) {
		return Ok(Value::Quantity(Quantity {
			amount: on_scale_of(&quantity.amount, from, to),
			factors: target,
		}));
	}
	let in_target = |factor: &Factor| {
		target
			.iter()
			.find(|wanted| wanted.unit.dimension == factor.unit.dimension)
			.map_or(factor.unit, |wanted| wanted.unit)
	};
	Ok(Value::Quantity(Quantity {
		amount: quantity.amount.mul(&scale(&quantity.factors, in_target)),
		factors: target,
	}))
}

/// Milliseconds in a day, a duration's `days` counted at their nominal length
pub(super) const DAY_MILLISECONDS: i128 = 86_400_000;
pub(super) const NANOS_PER_MILLISECOND: i128 = 1_000_000;

/// A duration of the time module in a time unit: `2 hours in minutes` is `120 min` (P36). None when the word names no
/// time unit; an error for months (no fixed length) or a part of a millisecond
pub fn duration_in(duration: &crate::time::Duration, unit_word: &str) -> Option<Node> {
	let target = target_unit(&Node::Symbol(unit_word.to_string()))?;
	if target.dimension != Dimension::Time {
		return Some(error(&format!(
			"DimensionError: {duration} cannot be converted to {}",
			target.name
		)));
	}
	let milliseconds = match duration_milliseconds(duration, target.name) {
		Ok(milliseconds) => milliseconds?,
		Err(message) => return Some(error(&message)),
	};
	let quantity = Quantity::of(milliseconds, unit_named("ms").expect("a unit"));
	Some(
		match convert(Value::Quantity(quantity), vec![Factor { unit: target, power: 1 }]) {
			Ok(Value::Quantity(quantity)) => Node::data(quantity),
			Err(Stop::Error(message)) => error(&message),
			_ => return None,
		},
	)
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

/// An amount of one unit read on the scale of another, each counted from its own zero: 40 °C is 104 °F
pub(super) fn on_scale_of(amount: &Rational, from: &Unit, to: &Unit) -> Rational {
	let steps = amount
		.mul(&Rational::integer(from.factor))
		.add(&Rational::integer(from.zero - to.zero));
	steps.mul(&Rational::new(BigInt::from(1), BigInt::from(to.factor)))
}

/// A unit counted from a zero of its own (°C, °F): its amounts add only to amounts of the same unit
pub(super) fn has_own_zero(quantity: &Quantity) -> bool {
	quantity.factors.iter().any(|factor| factor.unit.zero != 0)
}

/// The amount of a quantity in other units, as a factor: Π (unit/target)^power over its factors (1 km/h in m/s is 5/18)
pub(super) fn scale(factors: &[Factor], target: impl Fn(&Factor) -> &'static Unit) -> Rational {
	factors.iter().fold(Rational::integer(1), |product, factor| {
		let ratio = Rational::new(BigInt::from(factor.unit.factor), BigInt::from(target(factor).factor));
		let ratio = if factor.power > 0 {
			ratio
		} else {
			ratio.inverse().expect("unit factors are positive")
		};
		(0..factor.power.unsigned_abs()).fold(product, |product, _| product.mul(&ratio))
	})
}

/// `1 m / 3 m` is the number 1/3
pub(super) fn quotient_node(amount: &Rational) -> Node {
	Node::Number(Number::from_rational(amount.clone()))
}

pub(super) fn whole(amount: &Rational) -> Option<i64> {
	amount.is_integer().then(|| amount.numerator.to_i64()).flatten()
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

pub(super) fn finest_of<'a>(finest: &'a [&'static Unit]) -> impl Fn(&Factor) -> &'static Unit + 'a {
	move |factor| {
		finest
			.iter()
			.find(|unit| unit.dimension == factor.unit.dimension)
			.copied()
			.unwrap_or(factor.unit)
	}
}

/// An amount times the factors as one quantity: each dimension counts in its finest unit, its powers add
/// (`3 m * 2 m` is `6 m²`, `2 m * 3 kg` is `6 m·kg`, `10 km/h * 30 min` is `5 km`); no unit left is a plain number
pub(super) fn combine(amount: Rational, factors: Vec<Factor>) -> Evaluated {
	let finest = finest_units(&factors);
	let result: Vec<Factor> = signature(&factors)
		.into_iter()
		.map(|(dimension, power)| Factor {
			unit: finest
				.iter()
				.find(|unit| unit.dimension == dimension)
				.copied()
				.expect("a unit per dimension"),
			power,
		})
		.collect();
	let result = order_of_appearance(result, &factors);
	let amount = amount.mul(&scale(&factors, finest_of(&finest)));
	Ok(match whole(&amount) {
		Some(number) if result.is_empty() => Value::Number(number),
		_ => Value::Quantity(Quantity {
			amount,
			factors: result,
		}),
	})
}

/// The factors in the order their dimensions were first written: `2 m * 3 kg` is `m·kg`
pub(super) fn order_of_appearance(mut result: Vec<Factor>, written: &[Factor]) -> Vec<Factor> {
	let first = |dimension: Dimension| {
		written
			.iter()
			.position(|factor| factor.unit.dimension == dimension)
			.unwrap_or(usize::MAX)
	};
	result.sort_by_key(|factor| first(factor.unit.dimension));
	result
}

/// `(2 km)²` is `4 km²`
pub(super) fn power(base: Quantity, exponent: i64) -> Evaluated {
	let exponent = u32::try_from(exponent)
		.ok()
		.filter(|exponent| *exponent >= 1)
		.ok_or(Stop::Unsupported)?;
	let amount = (1..exponent).fold(base.amount.clone(), |amount, _| amount.mul(&base.amount));
	let factors = base
		.factors
		.iter()
		.map(|factor| Factor {
			power: factor.power * exponent as i32,
			..*factor
		})
		.collect();
	Ok(Value::Quantity(Quantity { amount, factors }))
}

/// Both amounts counted in the finer unit of each dimension: `m²` and `cm²` count in cm², `m²` and `m` do not mix
pub(super) fn aligned(left: &Quantity, right: &Quantity) -> Result<(Rational, Rational, Vec<Factor>), Stop> {
	if signature(&left.factors) != signature(&right.factors) {
		return fail(format!(
			"DimensionError: incompatible units: {} and {}",
			units_text(&left.factors),
			units_text(&right.factors)
		));
	}
	let finest = finest_units(&[left.factors.clone(), right.factors.clone()].concat());
	let target = finest_of(&finest);
	let factors: Vec<Factor> = left
		.factors
		.iter()
		.map(|factor| Factor {
			unit: target(factor),
			..*factor
		})
		.collect();
	let amount = |quantity: &Quantity| quantity.amount.mul(&scale(&quantity.factors, &target));
	Ok((amount(left), amount(right), factors))
}

/// Comparison of two quantities in their finer units: `3km == 3000m`, answered yes or no
pub(super) fn compare(left: Quantity, op: Op, right: Quantity) -> Evaluated {
	let (x, y) = match (left.single_unit(), right.single_unit()) {
		// `40°C > 100°F`: each read from its own zero
		(Some(unit), Some(other)) if unit.dimension == other.dimension => {
			(left.amount.clone(), on_scale_of(&right.amount, other, unit))
		}
		_ => aligned(&left, &right).map(|(x, y, _)| (x, y))?,
	};
	let order = x.compare(&y);
	let holds = match op {
		Op::Eq => order.is_eq(),
		Op::Ne => order.is_ne(),
		Op::Lt => order.is_lt(),
		Op::Gt => order.is_gt(),
		Op::Le => order.is_le(),
		_ => order.is_ge(),
	};
	Ok(Value::Bool(holds))
}

/// `x = 1 m; x = 2 s`: a variable keeps its dimension, as the static units pass checks (card units-reassign)
pub(super) fn same_dimension_kept(name: &str, earlier: &Value, given: &Value) -> Result<(), Stop> {
	let dimension = |value: &Value| match value {
		Value::Number(_) => Some((vec![], "a plain number".to_string())),
		Value::Quantity(quantity) => Some((signature(&quantity.factors), units_text(&quantity.factors))),
		_ => None,
	};
	let (Some((was, was_shown)), Some((is, is_shown))) = (dimension(earlier), dimension(given)) else {
		return Ok(());
	};
	match was.len() == is.len() && was.iter().all(|power| is.contains(power)) {
		true => Ok(()),
		false => fail(reassigned_dimension(name, &was_shown, was.is_empty(), &is_shown)),
	}
}

/// `x = 1200; x = 1.2kHz`: a variable keeps its dimension; a plain number becomes a quantity by its first value's unit
pub(crate) fn reassigned_dimension(name: &str, was: &str, was_plain: bool, given: &str) -> String {
	let hint = if was_plain {
		": give its first value a unit too"
	} else {
		""
	};
	format!("DimensionError: {name} was {was}, is given {given}{hint}")
}

pub(super) fn unitless_number(value: &Value) -> bool {
	matches!(value, Value::Number(_) | Value::Tolerance(Tolerance { unit: None, .. }))
}

/// A range or a value with tolerance as the closed span it covers, and its unit: `1950 ± 50 AD` is 1900 to 2000 AD
pub(super) fn span(value: &Value) -> Option<(i64, i64, Option<&'static str>)> {
	match value {
		Value::Range(range) => Some((range.from, range.to, Some(range.unit.name))),
		Value::Tolerance(tolerance) => Some((
			tolerance.value - tolerance.tolerance,
			tolerance.value + tolerance.tolerance,
			tolerance.unit.map(|unit| unit.name),
		)),
		_ => None,
	}
}

/// `1900 - 2000 AD == 1950 AD ± 50`: the same span in the same unit
pub(super) fn same_span(left: Value, op: Op, right: Value) -> Evaluated {
	let (Some((from, to, unit)), Some((other_from, other_to, other_unit))) = (span(&left), span(&right)) else {
		return Err(Stop::Unsupported);
	};
	if unit != other_unit {
		return fail(format!(
			"cannot compare spans in different units: {} and {}",
			unit.unwrap_or("no unit"),
			other_unit.unwrap_or("no unit")
		));
	}
	let same = (from, to) == (other_from, other_to);
	Ok(Value::Bool(same == (op == Op::Eq)))
}

pub(super) fn sum(left: Quantity, op: Op, right: Quantity) -> Evaluated {
	if left.factors != right.factors && (has_own_zero(&left) || has_own_zero(&right)) {
		return fail(format!(
			"{left} {op} {right}: add temperatures in one unit, convert one first: {right} as {}",
			units_text(&left.factors)
		));
	}
	let (x, y, factors) = aligned(&left, &right)?;
	let amount = if op == Op::Add { x.add(&y) } else { x.add(&y.neg()) };
	Ok(Value::Quantity(Quantity { amount, factors }))
}

/// `1950 ± 50`, `1950 cm ± 50` and `1950 ± 50 cm` all count in the unit that is present
pub(super) fn tolerance(center: Value, spread: Value) -> Evaluated {
	let plain = |quantity: &Quantity| {
		quantity
			.single_unit()
			.ok_or_else(|| Stop::Error(format!("{PLAIN_TOLERANCE}, not {quantity}")))
	};
	let counted =
		|amount: &Rational| whole(amount).ok_or_else(|| Stop::Error(format!("{WHOLE_TOLERANCE}, not {amount}")));
	let (value, tolerance, unit) = match (center, spread) {
		(Value::Number(value), Value::Number(tolerance)) => (value, tolerance, None),
		(Value::Quantity(value), Value::Number(tolerance)) => {
			(counted(&value.amount)?, tolerance, Some(plain(&value)?))
		}
		(Value::Number(value), Value::Quantity(tolerance)) => {
			(value, counted(&tolerance.amount)?, Some(plain(&tolerance)?))
		}
		(Value::Quantity(value), Value::Quantity(tolerance)) => {
			plain(&value)?;
			plain(&tolerance)?;
			let (value, tolerance, factors) = aligned(&value, &tolerance)?;
			(counted(&value)?, counted(&tolerance)?, Some(factors[0].unit))
		}
		_ => return fail("a tolerance applies to numbers and quantities"),
	};
	if tolerance < 0 {
		return fail(format!("the tolerance must not be negative: {tolerance}"));
	}
	Ok(Value::Tolerance(Tolerance { value, tolerance, unit }))
}

/// A plain number minus a quantity is a range in that unit: `1900 - 2000 AD`
pub(super) fn range(from: i64, end: Quantity) -> Evaluated {
	let to = whole(&end.amount).ok_or_else(|| Stop::Error(format!("a range counts whole numbers, not {end}")))?;
	if from > to {
		return fail(format!("descending range: {from} - {end}"));
	}
	Ok(Value::Range(Range {
		from,
		to,
		unit: end.single_unit().expect("guarded by arithmetic"),
	}))
}
