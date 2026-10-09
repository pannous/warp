//! `x ± r` at run time (card plus-minus, notes/plus_minus.md, decision P217): an interval with its value, flowing
//! through arithmetic with worst-case bounds. The run-time form is wasm_emitter/uncertain.rs; this is the value read back

use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;
use std::fmt;

/// Significant digits of a shown ± part; the value is shown to the same decimal place (decision P219)
const SHOWN_DIGITS: i32 = 2;
/// `y certainly < x`: the whole interval is below (P224b); also the builtin `certainly(y < x)` it lowers to
pub const CERTAINLY: &str = "certainly";
/// `y possibly < x`: some part of the interval is below
pub const POSSIBLY: &str = "possibly";
const CERTAINTY_WORDS: [&str; 2] = [CERTAINLY, POSSIBLY];
/// `x.value`, `x.low`, `x.high`, `x.uncertainty` of a ± value (wasm_emitter/uncertain.rs); a class method of one of
/// these names (Quantity's) is chosen by the receiver's class at run time (class_methods.rs)
pub const INTERVAL_FIELDS: [&str; 4] = ["value", "low", "high", "uncertainty"];
/// `5 ± 1σ`: a Gaussian ± (card plus-minus-gaussian), its spread one standard deviation; shown after its ± part
pub const SIGMA: &str = "σ";

/// Where a math word turns or jumps: an interval reaching `at + k·every` (just `at` when `every` is 0) has `value` as
/// its low (`high` false) or high bound
pub struct Extremum {
	pub at: f64,
	pub every: f64,
	pub high: bool,
	pub value: f64,
}

const fn turns(at: f64, every: f64, high: bool, value: f64) -> Extremum {
	Extremum { at, every, high, value }
}

const TAU: f64 = std::f64::consts::TAU;
const PI: f64 = std::f64::consts::PI;
const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
const LEAST_AT_ZERO: [Extremum; 1] = [turns(0.0, 0.0, false, 0.0)];
/// The math words an interval passes through (card plus-minus-playground, P217): √ ∛ abs and the libm functions of one
/// argument map the endpoints, an extremum or pole inside the interval is a bound. With the run-time function of each
/// (wasm_emitter/uncertain.rs)
pub const INTERVAL_WORDS: [(&str, &str, &[Extremum]); 23] = [
	("sqrt", "uncertain_sqrt", &[]), ("cbrt", "uncertain_cbrt", &[]),
	("abs", "uncertain_abs", &LEAST_AT_ZERO), ("fabs", "uncertain_fabs", &LEAST_AT_ZERO),
	("sin", "uncertain_sin", &[turns(HALF_PI, TAU, true, 1.0), turns(-HALF_PI, TAU, false, -1.0)]),
	("cos", "uncertain_cos", &[turns(0.0, TAU, true, 1.0), turns(PI, TAU, false, -1.0)]),
	("tan", "uncertain_tan", &[turns(HALF_PI, PI, true, f64::INFINITY), turns(HALF_PI, PI, false, f64::NEG_INFINITY)]),
	("asin", "uncertain_asin", &[]), ("acos", "uncertain_acos", &[]), ("atan", "uncertain_atan", &[]),
	("sinh", "uncertain_sinh", &[]), ("cosh", "uncertain_cosh", &[turns(0.0, 0.0, false, 1.0)]), ("tanh", "uncertain_tanh", &[]),
	("exp", "uncertain_exp", &[]), ("expm1", "uncertain_expm1", &[]),
	("log", "uncertain_log", &[]), ("log2", "uncertain_log2", &[]), ("log10", "uncertain_log10", &[]), ("log1p", "uncertain_log1p", &[]),
	("floor", "uncertain_floor", &[]), ("ceil", "uncertain_ceil", &[]), ("round", "uncertain_round", &[]), ("trunc", "uncertain_trunc", &[]),
];

/// A math word an interval passes through
pub fn maps_intervals(word: &str) -> bool {
	INTERVAL_WORDS.iter().any(|(name, _, _)| *name == word)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uncertain {
	pub value: f64,
	pub low: f64,
	pub high: f64,
	/// `5 ± 1σ`: low and high are one standard deviation from the value, and the ± part propagates in quadrature
	pub gaussian: bool,
}

impl Uncertain {
	/// From the run-time array [value, low, high], a Gaussian's [value, low, high, σ, its contributions…]
	pub fn from_parts(parts: &[f64]) -> Option<Uncertain> {
		match *parts {
			[value, low, high] => Some(Uncertain { value, low, high, gaussian: false }),
			[value, low, high, _, ..] => Some(Uncertain { value, low, high, gaussian: true }),
			_ => None,
		}
	}

	/// The node a reader (wasm_reader.rs, web.rs) makes of a Kind::Uncertain payload's parts
	pub fn read_node(parts: Option<Vec<f64>>) -> crate::node::Node {
		match parts.as_deref().and_then(Uncertain::from_parts) {
			Some(uncertain) => crate::node::Node::data(uncertain),
			None => crate::node::error("unreadable uncertain value"),
		}
	}

	/// The same uncertainty counted in a unit `factor` times larger (a positive factor)
	pub fn scaled(&self, factor: f64) -> Uncertain {
		Uncertain { value: self.value * factor, low: self.low * factor, high: self.high * factor, gaussian: self.gaussian }
	}

	/// The ± part: how far the interval reaches from the value, on its farther side
	pub fn radius(&self) -> f64 {
		(self.high - self.value).max(self.value - self.low)
	}
}

impl fmt::Display for Uncertain {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let radius = self.radius();
		let sigma = if self.gaussian { SIGMA } else { "" };
		if radius == 0.0 || !radius.is_finite() {
			return write!(f, "{} ± {}{sigma}", self.value, radius);
		}
		let mut place = radius.log10().floor() as i32 + 1 - SHOWN_DIGITS;
		// 0.0999… rounds up to 0.10, two digits of the next place
		if (radius / 10f64.powi(place)).round() >= 10f64.powi(SHOWN_DIGITS) {
			place += 1;
		}
		let decimals = (-place).max(0) as usize;
		let rounded = |x: f64| if place < 0 { x } else { (x / 10f64.powi(place)).round() * 10f64.powi(place) };
		write!(f, "{:.*} ± {:.*}{sigma}", decimals, rounded(self.value), decimals, rounded(radius))
	}
}

/// The spread of a Gaussian `5 ± 1σ` (the right side of its ±, `1*σ`), None for an interval's
pub fn gaussian_spread(right: &Node) -> Option<&Node> {
	match right.drop_meta() {
		Node::Key(spread, Op::Mul, sigma) if matches!(sigma.drop_meta(), Node::Symbol(name) if name == SIGMA) => Some(spread),
		_ => None,
	}
}

/// `y certainly < x` is `certainly(y < x)`, and `possibly` likewise: the builtins of wasm_emitter/uncertain.rs. The
/// parser reads it `(y certainly) < x` as a value and `y (certainly < x)` as a statement
pub fn lower_certainty(node: Node) -> Node {
	if !node.mentions_any(&CERTAINTY_WORDS) {
		return node;
	}
	let node = node.map_children(lower_certainty);
	let said = |word: &Node| CERTAINTY_WORDS.contains(&word.drop_meta().name().as_str());
	let (word, compared, op, right) = match node.drop_meta() {
		Node::Key(left, op, right) if op.is_ordering() => match left.drop_meta() {
			Node::List(items, Bracket::None, _) if matches!(items.as_slice(), [_, word] if said(word)) => (&items[1], &items[0], op, right),
			_ => return node,
		},
		// in parentheses as an `if` condition: `if (y certainly < x) then …`
		Node::List(items, Bracket::None | Bracket::Round, _) => match items.as_slice() {
			[compared, ordering] => match ordering.drop_meta() {
				Node::Key(word, op, right) if op.is_ordering() && said(word) => (word.as_ref(), compared, op, right),
				_ => return node,
			},
			_ => return node,
		},
		_ => return node,
	};
	let comparison = Node::Key(Box::new(compared.clone()), *op, right.clone());
	Node::List(vec![Node::Symbol(word.drop_meta().name()), comparison], Bracket::None, Separator::Space).with_meta_of(&node)
}

/// The word and the ordering of a lowered `certainly(y < x)`
pub fn certainty_parts(node: &Node) -> Option<(&str, &Node)> {
	match node.drop_meta() {
		Node::List(items, Bracket::None, _) => match items.as_slice() {
			[Node::Symbol(word), ordering] if CERTAINTY_WORDS.contains(&word.as_str()) && matches!(ordering.drop_meta(), Node::Key(_, op, _) if op.is_ordering()) => Some((word, ordering)),
			_ => None,
		},
		_ => None,
	}
}

/// P224b: `<`, `>`, `<=`, `>=` on a value the compiler knows to be ± must say how it compares; data read at run time
/// compares certainly (wasm_emitter/uncertain.rs)
pub fn check_orderings(program: &Node) -> Option<Diagnostic> {
	if !makes_intervals(program) {
		return None;
	}
	first_unsaid_ordering(program, &interval_names(program))
}

/// The first ordering of a known ± that is not said `certainly` or `possibly`
fn first_unsaid_ordering(node: &Node, intervals: &HashSet<String>) -> Option<Diagnostic> {
	match node.drop_meta() {
		Node::List(items, _, _) if matches!(items.first().map(|word| word.drop_meta().name()), Some(word) if CERTAINTY_WORDS.contains(&word.as_str())) => None,
		Node::Key(left, op, right) if op.is_ordering() && [left, right].iter().any(|side| is_interval(side, intervals)) => {
			Some(unsaid_ordering(node, left, *op, right))
		}
		Node::Key(left, _, right) => first_unsaid_ordering(left, intervals).or_else(|| first_unsaid_ordering(right, intervals)),
		Node::List(items, _, _) => items.iter().find_map(|item| first_unsaid_ordering(item, intervals)),
		_ => None,
	}
}

fn makes_intervals(program: &Node) -> bool {
	let mut found = false;
	program.visit(&mut |node| found |= matches!(node, Node::Key(_, Op::PlusMinus, _)));
	found
}

/// A program that makes ± values and orders values: an overlap may warn at run time
pub fn may_order_intervals(program: &Node) -> bool {
	let mut orders = false;
	program.visit(&mut |node| orders |= matches!(node, Node::Key(_, op, _) if op.is_ordering()));
	orders && makes_intervals(program)
}

/// The variables given a ± value, `y = 6 ± 1` or `y = x + 1` of such an x, until no more are found
fn interval_names(program: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	loop {
		let known = names.len();
		program.visit(&mut |node| {
			if let Node::Key(target, Op::Assign | Op::Define, value) = node {
				if let Node::Symbol(name) = target.drop_meta() {
					if is_interval(value, &names) {
						names.insert(name.clone());
					}
				}
			}
		});
		if names.len() == known {
			return names;
		}
	}
}

/// Known to be ± here: `6 ± 1`, a variable given one, arithmetic or a negation with one
fn is_interval(node: &Node, names: &HashSet<String>) -> bool {
	match node.drop_meta() {
		Node::Key(_, Op::PlusMinus, _) => true,
		Node::Symbol(name) => names.contains(name),
		Node::Key(left, Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Neg, right) => is_interval(left, names) || is_interval(right, names),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => is_interval(&items[0], names),
		_ => false,
	}
}

fn unsaid_ordering(node: &Node, left: &Node, op: Op, right: &Node) -> Diagnostic {
	let (left, right) = (left.serialize(), right.serialize());
	let written = format!("{left} {op} {right}");
	let readings = [CERTAINLY, POSSIBLY].map(|word| format!("{left} {word} {op} {right}"));
	let by_value = format!("{left}.value {op} {right}");
	Diagnostic::at(node, format!("{written} compares a ± value: an overlap is neither yes nor no"))
		.fix(format!("write `{}` (the whole interval), `{}` (some of it) or `{by_value}` (its value)", readings[0], readings[1]))
		.offer("the whole interval", &written, &readings[0])
		.offer("some of the interval", &written, &readings[1])
		.offer("its value", &written, &by_value)
}

/// The warning of a bare ordering whose operands turn out to overlap at run time (the compiler knew no ±)
pub fn overlap_warning(left: &Node, op: Op, right: &Node) -> String {
	let (left, right) = (left.serialize(), right.serialize());
	format!("{left} {op} {right}: the ± intervals overlap, which counts as no; write `{left} {CERTAINLY} {op} {right}`, `{left} {POSSIBLY} {op} {right}` or `{left}.value {op} {right}`")
}
