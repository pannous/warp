//! `use wasm "lib.wasm" as lib`: a WebAssembly component through foreign_call (src/components.rs): a Rust crate built for
//! wasm32-wasip2 (tests/fixtures/components/rust_demo, its WIT in wit/demo.wit) or any component with WIT exports
use crate::common::fails_with;
use std::path::PathBuf;
use warp::{ints, is};

/// A component written in WAT: `add: func(a: s64, b: s64) -> s64` and `echo: func(text: string) -> string`
const WAT_COMPONENT: &str = r#"(component
  (core module $m
    (memory (export "mem") 1)
    (global $heap (mut i32) (i32.const 1024))
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (local $at i32)
      (local.set $at (global.get $heap))
      (global.set $heap (i32.add (global.get $heap) (local.get 3)))
      (local.get $at))
    (func (export "add") (param i64 i64) (result i64) (i64.add (local.get 0) (local.get 1)))
    (func (export "echo") (param i32 i32) (result i32)
      (i32.store (i32.const 16) (local.get 0))
      (i32.store (i32.const 20) (local.get 1))
      (i32.const 16)))
  (core instance $i (instantiate $m))
  (func (export "add") (param "a" s64) (param "b" s64) (result s64) (canon lift (core func $i "add")))
  (func (export "echo") (param "text" string) (result string)
    (canon lift (core func $i "echo") (memory (core memory $i "mem")) (realloc (core func $i "realloc")))))"#;

fn wat_component() -> String {
	let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("wat_component.wasm");
	std::fs::write(&path, wat::parse_str(WAT_COMPONENT).unwrap()).unwrap();
	path.to_string_lossy().into_owned()
}

fn rust_component() -> String {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/components/rust_demo.wasm").to_string_lossy().into_owned()
}

#[test]
fn test_a_component_written_in_wat_is_called() {
	let path = wat_component();
	is!(&format!("use wasm \"{path}\" as demo; demo.add(40, 2)"), 42);
	is!(&format!("use wasm \"{path}\" as demo; demo.echo(\"hi there\")"), "hi there");
}

#[test]
fn test_a_rust_crate_built_as_a_component_is_called() {
	let path = rust_component();
	is!(&format!("use wasm \"{path}\"; rust_demo.fib(50)"), 12586269025i64);
	is!(&format!("use wasm \"{path}\" as r; r.words(\"a bc d\")"), warp::node::Node::List(vec!["a".into(), "bc".into(), "d".into()], warp::node::Bracket::Square, warp::node::Separator::Space));
	is!(&format!("use wasm \"{path}\" as r; s = r.stats_of(\"hello big world\"); [s.words, s.letters]"), ints(vec![3, 13]));
	is!(&format!("use wasm \"{path}\" as r; r.sides(\"square\")"), 4);
	is!(&format!("use wasm \"{path}\" as r; r.checked_sqrt(16.0)"), 4.0);
}

#[test]
fn test_component_failures_are_loud() {
	let path = rust_component();
	fails_with(&format!("use wasm \"{path}\" as r; r.checked_sqrt(-1.0)"), "no real square root of -1");
	fails_with(&format!("use wasm \"{path}\" as r; r.sides(\"triangle\")"), "triangle is none of circle, square");
	fails_with(&format!("use wasm \"{path}\" as r; r.nope(1)"), "exports no function nope; it exports fib");
	fails_with(&format!("use wasm \"{path}\" as r; r.fib(1, 2)"), "fib takes 1 arguments (n), got 2");
	fails_with("use wasm \"no/such.wasm\" as lib; lib.f(1)", "cannot load the component");
}
