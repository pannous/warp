//! `use rust_demo.wasm; rust_demo.counter(5)`: a component file used by its name, called by its stem (card g-_Xm4);
//! `use wasm "rust_demo.wasm" as rust` stays the long form, a core module file stays an import of its functions
use crate::is;

const RUST_DEMO: &str = "use tests/fixtures/components/rust_demo.wasm\n";

#[test]
fn a_component_file_is_used_by_its_name() {
	is!(&format!("{RUST_DEMO}rust_demo.fib(10)"), 55);
	is!(&format!("{RUST_DEMO}c = rust_demo.counter(5)\nc.increment(2)\nc.value()"), 7);
}

#[test]
fn the_long_form_and_core_modules_stay() {
	is!("use wasm \"tests/fixtures/components/rust_demo.wasm\" as rust\nrust.fib(10)", 55);
	is!("use tests/fixtures/wasm/counter.wasm; count_up(5)", 5);
}

// `use rust_demo.wasm as demo`, `use rust_demo as demo`: a component under a name of the program's choice; without
// `.wasm` where the loader finds the component file (card untitled-a)
#[test]
fn a_component_is_used_as_an_alias() {
	is!("use tests/fixtures/components/rust_demo.wasm as demo\ndemo.fib(10)", 55);
	is!("use tests/fixtures/components/rust_demo as demo\ndemo.fib(10)", 55);
	is!("use tests/fixtures/components/rust_demo\nrust_demo.fib(10)", 55);
}

// a component that is not there is an error naming those that are, natively those in its folder, in the page those
// build.sh made (components/names.txt; card playground-use)
#[test]
fn an_unknown_component_names_the_known_ones() {
	let outcome = warp::wasm_emitter::eval("use tests/fixtures/components/rust_dem.wasm as demo\ndemo.fib(10)").serialize();
	assert!(outcome.contains("no component") && outcome.contains("rust_dem.wasm;") && outcome.contains("rust_demo"), "{outcome}");
}

// `use nothere as x` that no component, warp module or C library resolves is one error naming all three, not a C
// library without functions (card use-nothere); a C library under an alias stays one
#[test]
fn an_alias_of_nothing_is_an_error() {
	let outcome = warp::wasm_emitter::eval("use tests/fixtures/components/nothere as x\nx.f(1)").serialize();
	assert!(outcome.contains("no module, component or library tests/fixtures/components/nothere; the components here: rust_demo"), "{outcome}");
	is!("use m as math\nsqrt(4)", 2);
}
