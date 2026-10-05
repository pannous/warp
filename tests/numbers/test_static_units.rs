//! Static units, stage 1 (notes/units_runtime.md): quantities computed at run time in variables, loops and branches. The
//! unit signature is checked at compile time, the amount runs in SI base units as an exact number, the result prints in
//! the finest written unit. (`1..3` excludes 3.)
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn test_quantities_accumulate_in_loops() {
	assert_eq!(shown("total = 0 m; for i in 1..3 { total += 5 m }; total"), "10 m");
	assert_eq!(shown("d = 0 km; for i in 1..4 { d = d + 250 m }; d"), "750 m");
	assert_eq!(shown("t = 0 h; n = 0; while n < 3 { t += 20 min; n += 1 }; t"), "60 min");
}

#[test]
fn test_quantities_in_branches_and_products() {
	assert_eq!(shown("c = 1; d = if c { 5 m } else { 3 m }; d"), "5 m");
	assert_eq!(shown("x = 2 km; t = 0 h; for i in 1..2 { t += 1 h }; x / t"), "2 km/h");
	assert_eq!(shown("a = 0 m; for i in 1..2 { a += 3 m }; a * 2 m"), "6 m²");
}

#[test]
fn test_dimension_errors_are_compile_errors() {
	fails_with("total = 0 m; for i in 1..3 { total += 5 s }; total", "DimensionError");
	fails_with("c = 1; d = if c { 5 m } else { 3 kg }; d", "DimensionError");
}

#[test]
fn test_unsupported_uses_stay_loud() {
	// print is covered since stage 3 (test_static_units_output.rs); lists are not yet
	fails_with("total = 0 m; for i in 1..3 { total += 5 m }; xs = [total, 1 m]; xs#1", "quantities compute only in constant expressions");
}

#[test]
fn test_compiling_a_quantity_result_is_refused_until_output_is_supported() {
	let compiled = warp::pipeline::compile("total = 0 m; for i in 1..3 { total += 5 m }; total");
	assert!(matches!(compiled, Err(warp::Node::Error(_))));
}
