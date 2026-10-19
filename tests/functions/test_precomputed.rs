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
