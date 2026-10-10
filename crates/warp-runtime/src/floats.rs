//! Floats as a run shows them (P119)

/// The one NaN every float a program shows has: 0x7ff8000000000000 (x86 makes 0xfff8…, ARM 0x7ff8…)
pub const CANONICAL_NAN: u64 = 0x7ff8_0000_0000_0000;

/// `x` as it may be observed (P119): a NaN of any sign or payload is the canonical one. Arithmetic inside a program
/// runs on raw NaNs; the host canonicalizes every float it reads from or hands out of a run (results, task values,
/// shared arrays, C arguments), and a program prints any NaN as `NaN`, so results stay bit-identical on every CPU
pub fn canonical_nan(x: f64) -> f64 {
	if x.is_nan() { f64::from_bits(CANONICAL_NAN) } else { x }
}
