//! `x ± r` at run time (card plus-minus, notes/plus_minus.md, decision P217): an interval with its value, flowing
//! through arithmetic with worst-case bounds. The run-time form is wasm_emitter/uncertain.rs; this is the value read back

use crate::node::{Bracket, Node};
use crate::operators::Op;
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
/// A math word and its run-time function, uncertain_<word>
macro_rules! interval_word {
	($word:literal, $extrema:expr) => {
		($word, concat!("uncertain_", $word), $extrema)
	};
}
/// The math words an interval passes through (card plus-minus-playground, P217): √ ∛ abs and the libm functions of one
/// argument map the endpoints, an extremum or pole inside the interval is a bound. With the run-time function of each
/// (wasm_emitter/uncertain.rs)
pub const INTERVAL_WORDS: [(&str, &str, &[Extremum]); 23] = [
	interval_word!("sqrt", &[]),
	interval_word!("cbrt", &[]),
	interval_word!("abs", &LEAST_AT_ZERO),
	interval_word!("fabs", &LEAST_AT_ZERO),
	interval_word!("sin", &[turns(HALF_PI, TAU, true, 1.0), turns(-HALF_PI, TAU, false, -1.0)]),
	interval_word!("cos", &[turns(0.0, TAU, true, 1.0), turns(PI, TAU, false, -1.0)]),
	interval_word!("tan", &[turns(HALF_PI, PI, true, f64::INFINITY), turns(HALF_PI, PI, false, f64::NEG_INFINITY)]),
	interval_word!("asin", &[]),
	interval_word!("acos", &[]),
	interval_word!("atan", &[]),
	interval_word!("sinh", &[]),
	interval_word!("cosh", &[turns(0.0, 0.0, false, 1.0)]),
	interval_word!("tanh", &[]),
	interval_word!("exp", &[]),
	interval_word!("expm1", &[]),
	interval_word!("log", &[]),
	interval_word!("log2", &[]),
	interval_word!("log10", &[]),
	interval_word!("log1p", &[]),
	interval_word!("floor", &[]),
	interval_word!("ceil", &[]),
	interval_word!("round", &[]),
	interval_word!("trunc", &[]),
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
		Uncertain {
			value: self.value * factor,
			low: self.low * factor,
			high: self.high * factor,
			gaussian: self.gaussian,
		}
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
		let rounded = |x: f64| {
			if place < 0 {
				x
			} else {
				(x / 10f64.powi(place)).round() * 10f64.powi(place)
			}
		};
		write!(f, "{:.*} ± {:.*}{sigma}", decimals, rounded(self.value), decimals, rounded(radius))
	}
}

/// The spread of a Gaussian `5 ± 1σ` (the right side of its ±, `1*σ`), None for an interval's
pub fn gaussian_spread(right: &Node) -> Option<&Node> {
	match right.drop_meta() {
		Node::Key(spread, Op::Mul, sigma) if matches!(sigma.drop_meta(), Node::Symbol(name) if name == SIGMA) => {
			Some(spread)
		}
		_ => None,
	}
}




/// The word and the ordering of a lowered `certainly(y < x)`
pub fn certainty_parts(node: &Node) -> Option<(&str, &Node)> {
	match node.drop_meta() {
		Node::List(items, Bracket::None, _) => match items.as_slice() {
			[Node::Symbol(word), ordering]
				if CERTAINTY_WORDS.contains(&word.as_str())
					&& matches!(ordering.drop_meta(), Node::Key(_, op, _) if op.is_ordering()) =>
			{
				Some((word, ordering))
			}
			_ => None,
		},
		_ => None,
	}
}









/// The warning of a bare ordering whose operands turn out to overlap at run time (the compiler knew no ±)
pub fn overlap_warning(left: &Node, op: Op, right: &Node) -> String {
	let (left, right) = (left.serialize(), right.serialize());
	format!("{left} {op} {right}: the ± intervals overlap, which counts as no; write `{left} {CERTAINLY} {op} {right}`, `{left} {POSSIBLY} {op} {right}` or `{left}.value {op} {right}`")
}
