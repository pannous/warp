//! Floats as a run shows them (P119)

/// The one NaN every float a program shows has: 0x7ff8000000000000 (x86 makes 0xfff8…, ARM 0x7ff8…)
pub const CANONICAL_NAN: u64 = 0x7ff8_0000_0000_0000;

/// `x` as it may be observed (P119): a NaN of any sign or payload is the canonical one. Arithmetic inside a program
/// runs on raw NaNs; the host canonicalizes every float it reads from or hands out of a run (results, task values,
/// shared arrays, C arguments), and a program prints any NaN as `NaN`, so results stay bit-identical on every CPU
pub fn canonical_nan(x: f64) -> f64 {
	if x.is_nan() { f64::from_bits(CANONICAL_NAN) } else { x }
}

/// The shortest decimal digits that read back as x (positive, finite, not 0) and the decimal exponent of the first
/// digit, as JavaScript and Python write floats: 1.5e-7 is (15, -7), 0.1 + 0.2 is (30000000000000004, -1)
pub fn shortest_digits(x: f64) -> (i64, i32) {
	let written = format!("{:e}", x.abs()); // Rust writes the shortest digits that read back
	let (mantissa, exponent) = written.split_once('e').unwrap_or((&written, "0"));
	(mantissa.replace('.', "").parse().unwrap_or(0), exponent.parse().unwrap_or(0))
}
