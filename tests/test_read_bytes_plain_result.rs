//! `read_bytes` reads a `main` that returns a plain number, and reports an unreadable result as an Error node, never a panic
#![cfg(feature = "native")]

use warp::extensions::numbers::Number;
use warp::node::Node;
use warp::wasm_reader::read_bytes;

const I64_RESULT: &str = r#"(module (func (export "main") (result i64) i64.const 42))"#;
const I32_RESULT: &str = r#"(module (func (export "main") (result i32) i32.const -7))"#;
const F64_RESULT: &str = r#"(module (func (export "main") (result f64) f64.const 2.5))"#;
const NULL_RESULT: &str = r#"(module (func (export "main") (result anyref) ref.null any))"#;
/// a GC value that is no Node struct: an i31ref
const I31_RESULT: &str = r#"(module (func (export "main") (result anyref) i32.const 5 ref.i31))"#;
/// a result type no Node can come from
const FUNCREF_RESULT: &str = r#"(module (func $f) (elem declare func $f) (func (export "main") (result funcref) ref.func $f))"#;
const NO_RESULT: &str = r#"(module (func (export "main")))"#;
const TWO_RESULTS: &str = r#"(module (func (export "main") (result i64 i64) i64.const 1 i64.const 2))"#;
const NO_ENTRY_POINT: &str = r#"(module (func (export "other") (result i64) i64.const 1))"#;

fn read(wat_source: &str) -> anyhow::Result<Node> {
	read_bytes(&wat::parse_str(wat_source).unwrap())
}

fn is_number(node: Node, expected: Number) -> bool {
	matches!(node, Node::Number(number) if number == expected)
}

#[test]
fn a_plain_i64_result_is_an_int() {
	assert!(is_number(read(I64_RESULT).unwrap(), Number::Int(42)));
}

#[test]
fn a_plain_i32_result_is_an_int() {
	assert!(is_number(read(I32_RESULT).unwrap(), Number::Int(-7)));
}

#[test]
fn a_plain_f64_result_is_a_float() {
	assert!(is_number(read(F64_RESULT).unwrap(), Number::Float(2.5)));
}

#[test]
fn a_null_result_is_empty() {
	assert!(matches!(read(NULL_RESULT).unwrap(), Node::Empty));
}

#[test]
fn a_main_without_a_result_is_empty() {
	assert!(matches!(read(NO_RESULT).unwrap(), Node::Empty));
}

#[test]
fn an_unreadable_result_is_an_error_not_a_panic() {
	for wat_source in [I31_RESULT, FUNCREF_RESULT, TWO_RESULTS, NO_ENTRY_POINT] {
		let outcome = std::panic::catch_unwind(|| read(wat_source));
		let outcome = outcome.unwrap_or_else(|_| panic!("read_bytes panicked on {wat_source}"));
		assert!(matches!(outcome, Err(_) | Ok(Node::Error(_))), "{wat_source} gave {outcome:?}");
	}
}
