//! `x ± σ` at run time (card plus-minus, notes/plus_minus.md): a value with a standard uncertainty that flows through
//! arithmetic with linear propagation, correlated per source. The run-time form is wasm_emitter/uncertain.rs; this is
//! the value read back

use std::fmt;

/// Significant digits of a shown uncertainty; the value is shown to the same decimal place
const SHOWN_DIGITS: i32 = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uncertain {
	pub value: f64,
	pub sigma: f64,
}

impl Uncertain {
	/// From the run-time array [value, source id, contribution, …]: σ is the root of the summed squared contributions
	pub fn from_parts(parts: &[f64]) -> Uncertain {
		let value = parts.first().copied().unwrap_or(f64::NAN);
		let sigma = parts.iter().skip(2).step_by(2).fold(0.0, |sum, contribution| sum + contribution * contribution).sqrt();
		Uncertain { value, sigma }
	}
}

impl fmt::Display for Uncertain {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.sigma == 0.0 || !self.sigma.is_finite() {
			return write!(f, "{} ± {}", self.value, self.sigma);
		}
		let place = self.sigma.abs().log10().floor() as i32 + 1 - SHOWN_DIGITS;
		let decimals = (-place).max(0) as usize;
		let rounded = |x: f64| if place < 0 { x } else { (x / 10f64.powi(place)).round() * 10f64.powi(place) };
		write!(f, "{:.*} ± {:.*}", decimals, rounded(self.value), decimals, rounded(self.sigma))
	}
}
