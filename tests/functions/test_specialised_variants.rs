// Precomputed paths (wiki/charged.md §3, package 3): a call of a pure function with some constant arguments gets its own
// variant with the constants in place (partial evaluation): `power(x, 3)` becomes x*(x*(x*1)), the program's value
// unchanged. Compiled modules only, like folding.
use warp::*;

const POWER: &str = "power(x, n) := if n == 0 then 1 else x * power(x, n - 1); ";

fn specialised(code: &str) -> String {
	let lowered = warp::pipeline::lower(code).expect("a program that needs a module");
	warp::folding::specialise_constant_arguments(lowered).serialize()
}

#[test]
fn a_constant_argument_unrolls_the_recursion() {
	let code = format!("{POWER}y = 2; power(y, 3) + power(y, 10)");
	let program = specialised(&code);
	assert!(!program.contains("(power y 3)") && !program.contains("(power y 10)"), "{program}");
	assert!(!program.contains("if 3==0"), "the condition on the constant is decided: {program}");
	// the chain of small variants is inlined: no call of power is left where it was called
	let precomputed = warp::folding::precompute(warp::pipeline::lower(&code).expect("a module")).serialize();
	let main = precomputed.rsplit("y=2").next().unwrap_or_default().to_string();
	assert!(!main.contains("power"), "{precomputed}");
	is!(&code, 1032);
	#[cfg(feature = "native")]
	{
		let module = warp::pipeline::compile(&code).expect("compiles");
		eq!(warp::wasm_reader::read_bytes(&module.bytes).expect("runs"), 1032);
	}
}

#[test]
fn an_unbounded_specialisation_keeps_the_call() {
	let code = "up(x, n) := if n > 100 then x else up(x + 1, n + 1); y = 0; up(y, 0)";
	let program = specialised(code);
	assert!(program.contains("(up y 0)"), "deeper than the bound: the original call stays: {program}");
	#[cfg(feature = "native")]
	{
		let module = warp::pipeline::compile(code).expect("compiles");
		eq!(warp::wasm_reader::read_bytes(&module.bytes).expect("runs"), 101);
	}
}
