//! The libm functions programs import from "m" (`cos(x)`, `pow(x, y)` …), for a store of any state: Rust's own f64
//! functions, so a standalone executable loads no C library (warp itself links the system's libm, ffi.rs)
use wasmtime::{Linker, Result};

pub const LIBM: &str = "m";
const UNARY: [(&str, fn(f64) -> f64); 11] = [("fabs", f64::abs), ("floor", f64::floor), ("ceil", f64::ceil), ("round", f64::round),
	("sqrt", f64::sqrt), ("sin", f64::sin), ("cos", f64::cos), ("tan", f64::tan), ("exp", f64::exp), ("log", f64::ln), ("log10", f64::log10)];
const BINARY: [(&str, fn(f64, f64) -> f64); 4] = [("fmin", f64::min), ("fmax", f64::max), ("fmod", fmod), ("pow", f64::powf)];

/// C's fmod: the remainder with the sign of the dividend, which is Rust's %
fn fmod(dividend: f64, divisor: f64) -> f64 {
	dividend % divisor
}

pub fn link_libm<T: 'static>(linker: &mut Linker<T>) -> Result<()> {
	for (name, function) in UNARY {
		linker.func_wrap(LIBM, name, move |x: f64| function(x))?;
	}
	for (name, function) in BINARY {
		linker.func_wrap(LIBM, name, move |x: f64, y: f64| function(x, y))?;
	}
	Ok(())
}

/// The names link_libm provides
pub fn libm_names() -> impl Iterator<Item = &'static str> {
	UNARY.iter().map(|(name, _)| *name).chain(BINARY.iter().map(|(name, _)| *name))
}
