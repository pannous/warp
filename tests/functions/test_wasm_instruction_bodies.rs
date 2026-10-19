//! Decision wasm-instruction-body: `name(params): type := wasm <instruction>` pushes the parameters in order and runs
//! the one instruction, whose result is the function's value.

use crate::common::fails_with;
use crate::is;

#[test]
fn float_instruction_bodies() {
	is!("g(a: float): float := wasm f64.floor; g(2.5)", 2.0);
	is!("m(a: float, b: float): float := wasm f64.min; m(2.5, 1.5)", 1.5);
	is!("q(a: float): float := wasm f64.sqrt; x = 16.0; q(x)", 4.0);
}

#[test]
fn int_instruction_bodies() {
	is!("h(a: int, b: int): int := wasm i64.rem_s; h(-7, 3)", -1);
	is!("p(a: int): int := wasm i64.popcnt; p(255)", 8);
	is!("t(a: float): int := wasm i64.trunc_f64_s; t(-3.7)", -3);
	is!("b(a: float): int := wasm i64.reinterpret_f64; b(1.0)", 4607182418800017408i64);
}

#[test]
fn unknown_instruction_is_an_error() {
	fails_with("z(a: float): float := wasm f64.bogus; z(1.0)", "wasm f64.bogus is no instruction");
}
