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
