//! card g_mnvA: a word can be an infix operator, with named parameters: `infix operator divides(d:int, n:int) := …`
//! makes `3 divides 12` a call of it; without parameters the operands are `left` and `right` (P48).
//! `infix divides(d, n) := …` is an alias with a got-it note
use crate::is;
use warp::normalize::{capture_hints, clear_shown_hints};

const DIVIDES: &str = "infix operator divides(d:int, n:int) := n % d == 0\n";

#[test]
fn a_word_infix_operator_with_parameters() {
	is!(&format!("{DIVIDES}3 divides 12"), true);
	is!(&format!("{DIVIDES}5 divides 12"), false);
	is!(&format!("{DIVIDES}count([d for d in 1..13 if d divides 12])"), 6);
	is!("infix operator divides(d:int,n:int) := { n % d == 0 }; 4 divides 12", true);
}

#[test]
fn a_word_infix_operator_without_parameters() {
	is!("infix operator times := left * right\n6 times 7", 42);
	is!("infix operator plus := a + b\n2 plus 3 * 4", 14);
}

#[test]
fn a_word_operator_is_still_a_word_elsewhere() {
	is!(&format!("{DIVIDES}dividesx = 3; dividesx + 1"), 4);
}

#[test]
fn infix_without_operator_is_an_alias() {
	clear_shown_hints();
	let (result, hints) = capture_hints(|| warp::wasm_emitter::eval("infix divides(d, n) := n % d == 0\n3 divides 9"));
	assert_eq!(result, warp::Node::True);
	assert!(hints.iter().any(|hint| hint.canonical == "infix operator divides"), "{hints:?}");
}

#[test]
fn an_assigned_filter_by_an_operator() {
	is!(&format!("{DIVIDES}xs = [4 6 8]; ys = xs where 4 divides it; ys"), warp::warp_parser::parse("[4 8]"));
	is!("xs = [4 6 8]; ys = xs where 4 + it > 9; ys", warp::warp_parser::parse("[6 8]"));
}
