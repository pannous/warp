// Precomputed paths (wiki/charged.md §3, package 3): a call of a pure function with some constant arguments gets its own
// variant with the constants in place (partial evaluation): `power(x, 3)` becomes x*(x*(x*1)), the program's value
// unchanged. Compiled modules only, like folding.
#[cfg(feature = "native")]
use crate::eq;
use crate::is;

const POWER: &str = "power(x, n) := if n == 0 then 1 else x * power(x, n - 1); ";

#[test]
fn a_constant_argument_unrolls_the_recursion() {
	let code = format!("{POWER}y = 2; power(y, 3) + power(y, 10)");
	// the chain of small variants is inlined: no call of power is left where it was called
	is!(&code, 1032);
	#[cfg(feature = "native")]
	{
		let module = warp::pipeline::compile(&code).expect("compiles");
		eq!(warp::run::wasm_reader::read_bytes(&module.bytes).expect("runs"), 1032);
	}
}

#[test]
fn an_unbounded_specialisation_keeps_the_call() {
	let code = "up(x, n) := if n > 100 then x else up(x + 1, n + 1); y = 0; up(y, 0)";
	#[cfg(feature = "native")]
	{
		let module = warp::pipeline::compile(code).expect("compiles");
		eq!(warp::run::wasm_reader::read_bytes(&module.bytes).expect("runs"), 101);
	}
}
