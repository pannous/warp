//! card rust-macros: a Rust macro call `name!(…)` / `name![…]`, the `!` glued to its name and its group, reads as the
//! warp form (decision rust-forms-welcome): `{}` placeholders filled in order, `vec!` a list, `assert!` an assert
use crate::is;
use warp::wasm_emitter::eval;
use warp::{parse, Node};

fn error_text(code: &str) -> String {
	match eval(code) {
		Node::Error(reason) => format!("{reason:?}"),
		other => panic!("{code}: expected an error, got {other:?}"),
	}
}

fn reads_as(rust: &str, warp: &str) {
	assert_eq!(parse(rust).serialize(), parse(warp).serialize(), "{rust} reads as {warp}");
}

#[test]
fn format_fills_the_placeholders_in_order() {
	is!("format!(\"a={} b={}\", 1, 2)", "a=1 b=2");
	is!("format!(\"plain\")", "plain");
	is!("let x = 3; format!(\"x={:?}\", x)", "x=3");
}

#[test]
fn format_reads_inline_names_and_positions() {
	is!("let x = 3; format!(\"x={x}\")", "x=3");
	is!("format!(\"{1} {0}\", \"a\", \"b\")", "b a");
	is!("let n = \"n\"; format!(\"{} {n} {}\", 1, 2)", "1 n 2");
}

#[test]
fn println_prints_the_formatted_text() {
	reads_as("println!(\"{} {}\", a, b)", "print(format(\"{} {}\", [a, b]))");
	reads_as("print!(\"{}\", a)", "print(format(\"{}\", [a]))");
	reads_as("println!(\"hi\")", "print(\"hi\")");
}

#[test]
fn vec_is_a_list() {
	is!("vec![1, 2, 3] == [1, 2, 3]", true);
	is!("let v = vec![1, 2, 3]; v.len()", 3);
}

#[test]
fn assert_macros_assert() {
	is!("assert!(1 == 1); 7", 7);
	is!("assert_eq!(2, 1 + 1); 7", 7);
	assert!(error_text("assert!(1 == 2)").contains("assertion failed"));
	assert!(error_text("assert_eq!(1, 2)").contains("assertion failed"));
	assert!(error_text("assert_ne!(1, 1)").contains("assertion failed"));
	assert!(error_text("assert!(1 == 2, \"bad {}\", 3)").contains("bad 3"));
}

#[test]
fn panic_macros_fail() {
	assert!(error_text("panic!(\"boom {}\", 1)").contains("boom 1"));
	assert!(error_text("todo!()").contains("not yet implemented"));
}

#[test]
fn another_macro_is_a_call() {
	is!("twice(x) := x * 2; twice!(4)", 8);
}

#[test]
fn a_spaced_bang_stays_not() {
	is!("yes and !(no)", true);
	is!("x = 3; x != 4", true);
}
