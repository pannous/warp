//! Div: whether a recursion demonstrably shrinks a parameter toward a guarded base case, so it terminates
//!
//! Total if one parameter p is a measure: every recursive call sits in a branch whose guard bounds p
//! by a number literal (`p<2 ? … : f(p-1)` bounds p ≥ 2 below) and passes `p - k` (or `p + k` toward
//! an upper bound) at p's position, k a positive literal. p then moves by at least k per call and the
//! call is only reached inside the bound, so every call chain is finite. p must not be assigned in the body.
//! Assumes finite numbers: a float NaN or ∞ argument can still run away (exact numbers are the default).
use super::*;

/// Which side of a literal a guard confines a parameter to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bound {
	/// p ≥ c or p > c: shrinking `p - k` terminates
	Lower,
	/// p ≤ c or p < c: growing `p + k` terminates
	Upper,
}

type Facts = Vec<(String, Bound)>;

/// `while 0 {…}` / `while false {…}` never runs its body
pub(super) fn never_true(condition: &Node) -> bool {
	match condition.drop_meta() {
		Node::Number(crate::extensions::numbers::Number::Int(0)) => true,
		Node::Symbol(word) => word == "false" || word == "False",
		Node::List(items, _, _) if items.len() == 1 => never_true(&items[0]),
		_ => false,
	}
}

pub(super) fn recursion_shrinks(name: &str, parameters: &[&str], body: &Node) -> bool {
	let measures = parameters.iter().enumerate().filter(|(_, parameter)| !assigns(body, parameter));
	measures
		.flat_map(|(index, parameter)| [(index, *parameter, Bound::Lower), (index, *parameter, Bound::Upper)])
		.any(|(index, parameter, bound)| calls_shrink(body, name, index, parameter, bound, &mut vec![]))
}

/// Every occurrence of `name` in `node` is a call that moves argument `index` toward the known `bound` of `parameter`
fn calls_shrink(node: &Node, name: &str, index: usize, parameter: &str, bound: Bound, facts: &mut Facts) -> bool {
	let in_branch = |branch: &Node, guard: &Node, holds: bool, facts: &mut Facts| {
		let before = facts.len();
		facts.extend(guard_bounds(guard, holds));
		let shrinks = calls_shrink(branch, name, index, parameter, bound, facts);
		facts.truncate(before);
		shrinks
	};
	if let Some((guard, then, otherwise)) = conditional(node) {
		return calls_shrink(guard, name, index, parameter, bound, facts)
			&& in_branch(then, guard, true, facts)
			&& otherwise.is_none_or(|otherwise| in_branch(otherwise, guard, false, facts));
	}
	match node.drop_meta() {
		Node::Symbol(symbol) => symbol != name, // the function used as a value: unknown calls
		Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(callee)) if callee == name) =>
		{
			let arguments = call_arguments(&items[1..]);
			let bounded = facts.iter().any(|(known, known_bound)| known == parameter && *known_bound == bound);
			bounded
				&& arguments.get(index).is_some_and(|argument| moves_toward(argument, parameter, bound))
				&& arguments.iter().all(|argument| calls_shrink(argument, name, index, parameter, bound, facts))
		}
		Node::List(items, _, _) => items.iter().all(|item| calls_shrink(item, name, index, parameter, bound, facts)),
		Node::Key(left, _, right) => {
			calls_shrink(left, name, index, parameter, bound, facts)
				&& calls_shrink(right, name, index, parameter, bound, facts)
		}
		Node::Error(inner) => calls_shrink(inner, name, index, parameter, bound, facts),
		_ => true,
	}
}

/// `f(a, b)` and `f a b` both list their arguments after the name; `f((a))` unwraps one group
pub(crate) fn call_arguments(arguments: &[Node]) -> Vec<&Node> {
	match arguments {
		[single] => match single.drop_meta() {
			Node::List(items, crate::node::Bracket::Round, _) if !items.is_empty() => items.iter().collect(),
			Node::Empty => vec![],
			_ => vec![single],
		},
		_ => arguments.iter().collect(),
	}
}

/// (guard, then, else) of `c ? a : b`, `if c then a else b`, `if c {a}`
fn conditional(node: &Node) -> Option<(&Node, &Node, Option<&Node>)> {
	match node.drop_meta() {
		Node::Key(guard, Op::Question, branches) => match branches.drop_meta() {
			Node::Key(then, Op::Colon, otherwise) => Some((&**guard, &**then, Some(&**otherwise))),
			_ => Some((&**guard, &**branches, None)),
		},
		Node::Key(if_then, Op::Else, otherwise) => {
			let (guard, then, _) = if_then_parts(if_then)?;
			Some((guard, then, Some(&**otherwise)))
		}
		_ => if_then_parts(node),
	}
}

fn if_then_parts(node: &Node) -> Option<(&Node, &Node, Option<&Node>)> {
	let Node::Key(if_guard, Op::Then, then) = node.drop_meta() else {
		return None;
	};
	match if_guard.drop_meta() {
		Node::Key(_, Op::If, guard) => Some((&**guard, &**then, None)),
		_ => None,
	}
}

/// Bounds on parameters known when `guard` evaluates to `holds`
fn guard_bounds(guard: &Node, holds: bool) -> Facts {
	match guard.drop_meta() {
		Node::List(items, _, _) if items.len() == 1 => guard_bounds(&items[0], holds),
		Node::Key(left, Op::And, right) if holds => [guard_bounds(left, true), guard_bounds(right, true)].concat(),
		Node::Key(left, Op::Or, right) if !holds => [guard_bounds(left, false), guard_bounds(right, false)].concat(),
		Node::Key(left, op @ (Op::Lt | Op::Le | Op::Gt | Op::Ge), right) => {
			let (parameter, below_literal) = match (left.drop_meta(), right.drop_meta()) {
				(Node::Symbol(parameter), Node::Number(limit)) if is_finite(limit) => {
					(parameter, matches!(op, Op::Lt | Op::Le))
				}
				(Node::Number(limit), Node::Symbol(parameter)) if is_finite(limit) => {
					(parameter, matches!(op, Op::Gt | Op::Ge))
				}
				_ => return vec![],
			};
			let bound = if below_literal == holds { Bound::Upper } else { Bound::Lower };
			vec![(parameter.clone(), bound)]
		}
		_ => vec![],
	}
}

/// `p - k` toward a lower bound, `p + k` toward an upper bound, k a positive literal
fn moves_toward(argument: &Node, parameter: &str, bound: Bound) -> bool {
	match argument.drop_meta() {
		Node::List(items, _, _) if items.len() == 1 => moves_toward(&items[0], parameter, bound),
		Node::Key(left, op @ (Op::Sub | Op::Add), step) => {
			let Node::Number(step) = step.drop_meta() else {
				return false;
			};
			let Node::Symbol(moved) = left.drop_meta() else {
				return false;
			};
			let direction = if *op == Op::Sub { -sign(step) } else { sign(step) };
			moved == parameter && direction == if bound == Bound::Lower { -1 } else { 1 }
		}
		_ => false,
	}
}

fn is_finite(number: &crate::extensions::numbers::Number) -> bool {
	use crate::extensions::numbers::Number;
	match number {
		Number::Int(_) | Number::Quotient(..) | Number::BigQuotient(_) | Number::BigInt(_) => true,
		Number::Float(value) => value.is_finite(),
		_ => false,
	}
}

/// Sign of a finite real literal, 0 when unknown
fn sign(number: &crate::extensions::numbers::Number) -> i8 {
	use crate::extensions::numbers::Number;
	let positive = match number {
		Number::Int(value) => *value > 0,
		Number::Float(value) => value.is_finite() && *value > 0.0,
		Number::Quotient(numerator, denominator) => (*numerator > 0) == (*denominator > 0) && *numerator != 0,
		Number::BigQuotient(q) => !q.is_zero() && !q.is_negative(),
		_ => return 0,
	};
	let negative = match number {
		Number::Int(value) => *value < 0,
		Number::Float(value) => value.is_finite() && *value < 0.0,
		Number::Quotient(numerator, denominator) => (*numerator > 0) != (*denominator > 0) && *numerator != 0,
		Number::BigQuotient(q) => q.is_negative(),
		_ => false,
	};
	if positive {
		1
	} else if negative {
		-1
	} else {
		0
	}
}

/// The body writes `parameter` (`p = …`, `p += …`, `p++`): its value at a call is no longer the guarded one
fn assigns(node: &Node, parameter: &str) -> bool {
	match node.drop_meta() {
		Node::Key(left, op, right) => {
			let writes = matches!(
				op,
				Op::Assign
					| Op::Define | Op::AddAssign
					| Op::SubAssign | Op::MulAssign
					| Op::DivAssign | Op::ModAssign
					| Op::PowAssign | Op::AndAssign
					| Op::OrAssign | Op::XorAssign
					| Op::Inc | Op::Dec
			);
			(writes && matches!(left.drop_meta(), Node::Symbol(name) if name == parameter))
				|| (matches!(op, Op::Inc | Op::Dec)
					&& matches!(right.drop_meta(), Node::Symbol(name) if name == parameter))
				|| assigns(left, parameter)
				|| assigns(right, parameter)
		}
		Node::List(items, _, _) => items.iter().any(|item| assigns(item, parameter)),
		Node::Error(inner) => assigns(inner, parameter),
		_ => false,
	}
}
