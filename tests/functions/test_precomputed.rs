// Precomputed and precompiled paths (wiki/charged.md §3, released 2026-10-05, package 3): the compiler may compute a
// pure part earlier; whatever reads shared state or has effects runs at every call.
#[cfg(feature = "native")]
use crate::eq;
use crate::is;
use warp::*;

#[test]
fn a_function_reading_a_global_is_not_pure() {
	is!("global y=1; f() := y*2; effects of f", Node::Symbol("State".into()));
	is!("y=3; def z(){ global y; y*y }; effects of z", Node::Symbol("State".into()));
	is!("y=1; f() := y*2; effects of f", Node::Symbol("Pure".into()));
	is!("global y=1; f(y) := y*2; effects of f", Node::Symbol("Pure".into()));
}

#[test]
#[cfg(feature = "native")] // reads the compiled module with wasmtime
fn a_pure_call_with_constant_arguments_is_folded_in_a_compiled_module() {
	let module =
		warp::pipeline::compile("def fib(n): n<2 ? n : fib(n-1)+fib(n-2); x = fib(20); x+1").expect("compiles");
	eq!(warp::run::wasm_reader::read_bytes(&module.bytes).expect("runs"), 6766);
}

// card front-prints: the folding compiles its own module of the foldable definitions alone; what that module's compile
// says about them (`Round` names nothing, the enum declaration being outside it) never reaches the program's compile
#[test]
#[cfg(feature = "native")] // reads the compiled module with wasmtime
fn folding_says_nothing_of_its_own() {
	warp::errors::diagnostic::take_warnings();
	let code = "enum Shape { Round, Square }\nshape_of(n: int): Shape := if n > 0 then Shape.Round else Shape.Square\nshape_of(1)";
	let module = warp::pipeline::compile(code).expect("compiles");
	let warnings: Vec<String> = warp::errors::diagnostic::take_warnings().iter().map(|w| w.to_string()).collect();
	assert!(warnings.is_empty(), "{warnings:?}");
	eq!(warp::run::wasm_reader::read_bytes(&module.bytes).expect("runs"), Node::Symbol("Round".into()));
}
