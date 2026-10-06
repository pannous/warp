//! `use wasm "lib.wasm" as lib`: a WebAssembly component through foreign_call (src/components.rs): a Rust crate built for
//! wasm32-wasip2 (tests/fixtures/components/rust_demo, its WIT in wit/demo.wit) or any component with WIT exports
use crate::common::fails_with;
use std::path::PathBuf;
use warp::ints;
use crate::is;

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

/// A WIT resource stays in the component: warp holds a handle, its methods run in the component (card wit-resources)
#[test]
fn test_resources_of_a_component_are_handles() {
	let path = rust_component();
	let using = |code: &str| format!("use wasm \"{path}\" as r; {code}");
	is!(&using("c = r.counter(5); c.increment(2); c.value()"), 7);
	is!(&using("c = r.Counter(5); [c.increment(3), c.value()]"), ints(vec![8, 8]));
	// a static function takes handles and gives a new one; a list of borrowed handles
	is!(&using("a = r.counter(2); b = r.counter(3); m = r.merged(a, b); m.value()"), 5);
	is!(&using("a = r.counter(2); b = r.counter(3); r.total([a, b])"), 5);
	// warp's own methods stay warp's on a component's plain results
	is!(&using("s = r.stats_of(\"ab c\"); s.words * 10"), 20);
}

#[test]
fn test_resource_failures_are_loud() {
	let path = rust_component();
	let using = |code: &str| format!("use wasm \"{path}\" as r; {code}");
	fails_with(&using("c = r.counter(1); c.nope()"), "counter#1: a counter has no method nope");
	fails_with(&using("r.total([r.stats_of(\"x\")])"), "is no handle of a resource");
}

/// a field of a call result is the field of the record the component gave (card foreign-results)
#[test]
fn test_a_field_of_a_component_result_is_read() {
	let path = rust_component();
	is!(&format!("use wasm \"{path}\" as r; r.stats_of(\"hello big world\").letters"), 13);
}

/// a program's handles are numbered from 1 whatever other threads (parallel tests, other programs) created
#[test]
fn test_handles_of_another_thread_do_not_shift_the_numbering() {
	let path = rust_component();
	let using = move |code: &str| format!("use wasm \"{path}\" as r; {code}");
	let other = using("a = r.counter(1); b = r.counter(2); a.value() + b.value()");
	std::thread::spawn(move || { warp::wasm_emitter::eval(&other); }).join().unwrap();
	fails_with(&using("c = r.counter(1); c.nope()"), "counter#1: a counter has no method nope");
}
