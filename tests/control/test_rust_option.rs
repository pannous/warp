//! card rust-option: Rust's Option and loop read as warp (decision rust-forms-welcome): `Some(v)` is the value, `None`
//! ø, `if let Some(v) = o` and a `match` of Some/None arms test for ø, `unwrap_or(d)` is `?? d`, `Option<T>` is `T?`,
//! `loop { … }` is `while true { … }`
use crate::is;
use warp::wasm_emitter::eval;
use warp::Node;

fn error_text(code: &str) -> String {
	match eval(code) {
		Node::Error(reason) => format!("{reason:?}"),
		other => panic!("{code}: expected an error, got {other:?}"),
	}
}

#[test]
fn some_is_the_value_none_is_nothing() {
	is!("let o = Some(3); o.unwrap()", 3);
	is!("let o: Option<i32> = None; o.unwrap_or(0)", 0);
	is!("let o = Some(3); o.unwrap_or(0)", 3);
	is!("let o = Some(3); o.is_some()", true);
	is!("let o: Option<i32> = None; o.is_none()", true);
	is!("let o = Some(0); o.is_some()", true);
	assert!(error_text("let o: Option<i32> = None; o.expect(\"no value\")").contains("no value"));
}

#[test]
fn if_let_some_binds_the_value() {
	is!("let o = Some(3); if let Some(v) = o { v * 2 } else { 0 }", 6);
	is!("let o: Option<i32> = None; if let Some(v) = o { v * 2 } else { 0 }", 0);
	is!("let o = Some(0); if let Some(v) = o { v + 1 } else { 9 }", 1);
}

#[test]
fn a_match_of_some_and_none() {
	is!("let o = Some(3); match o { Some(v) => v + 1, None => 0 }", 4);
	is!("let o: Option<i32> = None; match o { Some(v) => v + 1, None => 0 }", 0);
	is!("let o = Some(3); match o { None => 0, Some(v) => v }", 3);
}

#[test]
fn a_function_returns_an_option() {
	is!("fn half(x: i32) { if x % 2 == 0 { Some(x / 2) } else { None } } half(4).unwrap_or(0)", 2);
	is!("fn half(x: i32) { if x % 2 == 0 { Some(x / 2) } else { None } } half(3).unwrap_or(0)", 0);
}

#[test]
fn option_type_is_optional() {
	is!("let o: option<int> = ø; o ?? 5", 5);
}

#[test]
fn loop_runs_until_break() {
	is!("let mut i = 0; loop { i += 1; if i == 3 { break; } } i", 3);
}

#[test]
fn a_programs_own_option_method_keeps_its_meaning() {
	is!("expect(x, y) := x + y; o = 3; o.expect(5)", 8);
	is!("unwrap_or(x, y) := x * y; o = Some(3); o.unwrap_or(5)", 15);
	// a sum type's own Some is a class with a payload field
	is!("type Option[T] = Some(T) | None; s = Some(4); s#1 + s.value", 8);
}
