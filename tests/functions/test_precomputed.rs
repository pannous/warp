// Precomputed and precompiled paths (wiki/charged.md §3, released 2026-10-05, package 3): the compiler may compute a
// pure part earlier; whatever reads shared state or has effects runs at every call.
use warp::*;

#[test]
fn a_function_reading_a_global_is_not_pure() {
	is!("global y=1; f() := y*2; effects of f", Node::Symbol("State".into()));
	is!("y=3; def z(){ global y; y*y }; effects of z", Node::Symbol("State".into()));
	is!("y=1; f() := y*2; effects of f", Node::Symbol("Pure".into()));
	is!("global y=1; f(y) := y*2; effects of f", Node::Symbol("Pure".into()));
}

fn folded(code: &str) -> String {
	let lowered = warp::pipeline::lower(code).expect("a program that needs a module");
	warp::folding::fold_constant_calls(lowered).serialize()
}

#[test]
fn a_pure_call_with_constant_arguments_is_folded_in_a_compiled_module() {
	let fib = folded("def fib(n): n<2 ? n : fib(n-1)+fib(n-2); x = fib(20); x+1");
	assert!(fib.contains("6765") && !fib.contains("(fib 20)"), "{fib}");
	let module = warp::pipeline::compile("def fib(n): n<2 ? n : fib(n-1)+fib(n-2); x = fib(20); x+1").expect("compiles");
	eq!(warp::wasm_reader::read_bytes(&module.bytes).expect("runs"), 6766);
	let texts = folded("def greet(name): \"hi \" + name; s = greet(\"ann\"); s");
	assert!(texts.contains("hi ann"), "{texts}");
}

#[test]
fn effects_shared_state_and_failures_are_never_folded() {
	assert!(folded("def r(n): random_below(n); x = r(5); x").contains("(r 5)"));
	assert!(folded("global k=2; def f(x): x*k; y = f(3); y").contains("(f 3)"));
	assert!(folded("def d(x): 10/x; y = d(0); y").contains("(d 0)"));
}
