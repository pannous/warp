//! Card standalone-math-units step 3, decision foreign-signature: the math words warp computes itself are lib/math.warp
//! lines `floor(x: number): int from warp`; their signatures come from there, not from a Rust table
use crate::is;
use warp::Node;

#[test]
fn a_builtin_words_signature_is_its_line() {
	let lines: Vec<&str> = warp::ffi::math_foreign_lines(warp::ffi::BUILTIN_LIBRARY).map(|line| line.name).collect();
	assert!(lines.contains(&"floor") && lines.contains(&"max"), "{lines:?}");
	is!("floor.signature", Node::Text("(x:number) -> int".into()));
	is!("sin.signature", Node::Text("(x:float) -> float".into()));
}

/// warp's floor is the bare word, C's float floor stays the library's (interviewer)
#[test]
fn a_builtin_line_declares_and_imports_nothing() {
	is!("include math; floor(2.5) + max(1, 5, 3)", 7);
	is!("sin(x: float): float from warp\nsin(0)", 0);
	is!("type(floor(2.5))", "int");
}
