// Precomputed paths (wiki/charged.md §3, package 3): a main-level variable bound once to a constant before a function's
// definition and never changed is read as that constant in the body (`k = 3; def f(x){x+k}` → `x+3`), and Int
// arithmetic left constant is computed. A `global` variable, a changed one or a non-constant value stays a variable.
// Compiled modules only, like folding; the program's value is unchanged.
use warp::*;

fn inlined(code: &str) -> String {
	let lowered = warp::pipeline::lower(code).expect("a program that needs a module");
	warp::folding::fold_constant_free_variables(lowered).serialize()
}

#[cfg(feature = "native")]
fn compiled(code: &str) -> Node {
	let module = warp::pipeline::compile(code).expect("compiles");
	warp::wasm_reader::read_bytes(&module.bytes).expect("runs")
}

/// The same value through eval and, natively, through the compiled (precomputed) module
fn same_value(code: &str, value: i64) {
	is!(code, value);
	#[cfg(feature = "native")]
	eq!(compiled(code), value);
}

#[test]
fn a_stable_constant_is_inlined() {
	let code = "k = 3; def f(x){x+k}; f(1) + f(2)";
	let program = inlined(code);
	assert!(program.contains("x+3") && !program.contains("x+k"), "{program}");
	same_value(code, 9);
}

#[test]
fn constant_arithmetic_is_computed_after_inlining() {
	let code = "y = 3; def f(x){y*y + x}; f(1)";
	let program = inlined(code);
	assert!(program.contains("9+x") && !program.contains("y*y"), "{program}");
	same_value(code, 10);
	let code = "def f(x){x + 3*4}; f(1)";
	let program = inlined(code);
	assert!(program.contains("x+3*4"), "a body without inlined variable keeps its form: {program}");
}

#[test]
fn a_condition_on_a_constant_is_decided() {
	let code = "limit = 10; def f(x){ if limit > 2 then x else 0 }; f(5)";
	let program = inlined(code);
	assert!(!program.contains("limit>2") && !program.contains("10>2"), "{program}");
	same_value(code, 5);
	let code = "limit = 10; def capped(x){ if x > limit then limit else x }; capped(3) + capped(30)";
	let program = inlined(code);
	assert!(program.contains("x>10") && !program.contains("x>limit"), "{program}");
	// compiled only: eval of a captured variable as a branch value traps today (`then limit`, cast failure)
	#[cfg(feature = "native")]
	eq!(compiled(code), 13);
}

#[test]
fn a_global_variable_stays() {
	let code = "k = 3; def f(x){global k; x+k}; k = 4; f(1)";
	let program = inlined(code);
	assert!(program.contains("x+k"), "{program}");
	same_value(code, 5);
	// first bound after the definition: late binding makes it global, the call reads it
	let code = "def f(x){x+k}; k = 3; f(1)";
	let program = inlined(code);
	assert!(program.contains("x+k"), "{program}");
	same_value(code, 4);
}

#[test]
fn a_changed_variable_stays() {
	let code = "k = 2; k = 3; def f(x){x+k}; f(1)";
	let program = inlined(code);
	assert!(program.contains("x+k"), "{program}");
	same_value(code, 4);
	let code = "k = 0; for i in 1 to 3 { k += i }; def f(x){x+k}; f(1)";
	let program = inlined(code);
	assert!(program.contains("x+k"), "{program}");
	same_value(code, 7);
}

#[test]
fn a_non_constant_value_stays() {
	let code = "n = 2; k = n*3; def f(x){x+k}; f(1)";
	let program = inlined(code);
	assert!(program.contains("x+k"), "{program}");
	same_value(code, 7);
}

#[test]
fn a_parameter_or_a_key_of_the_same_name_stays() {
	let code = "x = 3; def f(x){x+1}; f(1)";
	let program = inlined(code);
	assert!(program.contains("x+1") && !program.contains("3+1"), "{program}");
	same_value(code, 2);
	let code = "k = 3; def f(x){m={k:1}; m.k + x}; f(1)";
	let program = inlined(code);
	assert!(program.contains("k:1"), "{program}");
	same_value(code, 2);
}

#[test]
fn a_constant_text_is_inlined() {
	let code = "greeting = \"hi \"; def f(name){greeting + name}; f(\"you\")";
	let program = inlined(code);
	assert!(!program.contains("greeting+name"), "{program}");
	is!(code, "hi you");
	#[cfg(feature = "native")]
	eq!(compiled(code), "hi you");
}
