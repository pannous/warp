//! `x ± r` at run time (card plus-minus, notes/plus_minus.md, decision P217): an interval with its value, flowing
//! through arithmetic with worst-case bounds. The run-time form is wasm_emitter/uncertain.rs; this is the value read back

use std::fmt;

/// Significant digits of a shown ± part; the value is shown to the same decimal place (decision P219)
const SHOWN_DIGITS: i32 = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uncertain {
	pub value: f64,
	pub low: f64,
	pub high: f64,
}

impl Uncertain {
	/// From the run-time array [value, low, high]
	pub fn from_parts(parts: &[f64]) -> Option<Uncertain> {
		match *parts {
			[value, low, high] => Some(Uncertain { value, low, high }),
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

	/// The ± part: how far the interval reaches from the value, on its farther side
	pub fn radius(&self) -> f64 {
		(self.high - self.value).max(self.value - self.low)
	}
}

impl fmt::Display for Uncertain {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let radius = self.radius();
		if radius == 0.0 || !radius.is_finite() {
			return write!(f, "{} ± {}", self.value, radius);
		}
		let place = radius.log10().floor() as i32 + 1 - SHOWN_DIGITS;
		let decimals = (-place).max(0) as usize;
		let rounded = |x: f64| if place < 0 { x } else { (x / 10f64.powi(place)).round() * 10f64.powi(place) };
		write!(f, "{:.*} ± {:.*}", decimals, rounded(self.value), decimals, rounded(radius))
	}
}
