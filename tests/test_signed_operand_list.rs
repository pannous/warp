//! D13 (user, 2026-10-03): `1 -1` (a space before the sign, none after) asks list or arithmetic; unanswered it takes
//! the list `[1 -1]` with a warning; the arithmetic is written `1 - 1`
use warp::diagnostic::{take_assumptions, with_warning_mode, WarningMode};
use warp::node::ints;
use warp::*;

#[test]
fn a_glued_sign_after_a_space_is_a_list_by_default() {
	is!("1 -1", ints(vec![1, -1]));
	take_assumptions();
	warp::wasp_parser::parse("1 -1");
	let assumed = take_assumptions();
	assert!(assumed.iter().any(|warning| warning.message.contains("taking the list")), "{assumed:?}");
	is!("[1 -1]", ints(vec![1, -1]));
	is!("[1 -1 2]", ints(vec![1, -1, 2]));
	is!("[1 +1]", ints(vec![1, 1]));
	is!("x=[3 -1]; x#2", -1);
}

#[test]
fn an_unbracketed_assignment_of_the_list_asks_once_more() {
	crate::common::fails_with("x=1 -1; x", "is `x=1 -1` one list or separate statements?");
}

#[test]
fn spaced_or_glued_operators_stay_arithmetic() {
	is!("1 - 1", 0);
	is!("1-1", 0);
	is!("1+ 1", 2);
	is!("x=5;x -1", 4);
}

#[test]
fn each_reading_has_its_explicit_form() {
	is!("1 - 1", 0);
	is!("[3 - 1]", ints(vec![2]));
	is!("[3 -1]", ints(vec![3, -1]));
}

#[test]
fn under_strict_the_default_is_an_error() {
	assert!(matches!(with_warning_mode(WarningMode::Error, || warp::wasm_emitter::eval("1 -1")), Node::Error(_)));
}
