//! card match (user): `rest =>` catches what nothing else matches, a soft synonym of `other` with a got-it note; a
//! variable named rest stays a variable, and an arm reading rest binds it
use crate::is;
use warp::normalize::{capture_hints, set_hint_mode, HintMode};
use warp::wasm_emitter::eval;

#[test]
fn rest_catches_the_rest_with_a_note() {
	set_hint_mode(HintMode::Always);
	let (value, hints) = capture_hints(|| eval("match 3 { [a, _] => a + 1; rest => 0 }"));
	assert_eq!(value, warp::Node::int(0));
	assert!(hints.iter().any(|hint| hint.original == "rest" && hint.canonical == "other"), "{hints:?}");
	is!("match [3, 4] { [a, _] => a + 1; rest => 0 }", 4);
}

#[test]
fn rest_the_arm_reads_is_the_matched_value() {
	is!("match 3 { 1 => 0; rest => rest + 1 }", 4);
}

#[test]
fn a_variable_named_rest_stays_a_variable() {
	is!("rest = 2; match 2 { rest => \"two\"; _ => \"no\" }", "two");
	is!("rest = 2; match 3 { rest => \"two\"; _ => \"no\" }", "no");
}
