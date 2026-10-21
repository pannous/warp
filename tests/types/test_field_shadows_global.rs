//! card bare-name: a bare name that is both a field of the receiver's class and a global variable reads the field
//! (P115, one-method-form), with a hint naming the field read and the shadowed global, in both method forms
use warp::normalize::{capture_hints, clear_shown_hints};
use warp::wasm_emitter::eval;

const SHADOWED: &str = "the global n is shadowed";

fn shadow_hints(code: &str) -> (String, Vec<(String, String)>) {
	clear_shown_hints();
	let (value, hints) = capture_hints(|| eval(code));
	let shadowing = hints.into_iter().filter(|hint| hint.reason.contains(SHADOWED));
	(value.serialize(), shadowing.map(|hint| (hint.original, hint.canonical)).collect())
}

#[test]
fn an_outside_method_reads_the_field_with_a_hint() {
	let (value, hints) = shadow_hints("n = 10\nclass P { n: int }\ndef x(p: P) := n + 1\nx(P(3))");
	assert_eq!(value, "4");
	assert_eq!(hints, vec![("n".to_string(), "p.n".to_string())]);
}

#[test]
fn a_class_body_method_reads_the_field_with_a_hint() {
	let (value, hints) = shadow_hints("n = 10\nclass Q { n: int; def y() := n + 1 }\nQ(5).y()");
	assert_eq!(value, "6");
	assert_eq!(hints, vec![("n".to_string(), "self.n".to_string())]);
}

#[test]
fn a_field_without_a_global_of_its_name_reads_silently() {
	let (value, hints) = shadow_hints("class P { n: int }\ndef x(p: P) := n + 1\nx(P(3))");
	assert_eq!(value, "4");
	assert!(hints.is_empty(), "{hints:?}");
}
