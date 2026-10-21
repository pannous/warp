//! card implicit-self (user 2026-10-15): self and this are needed only where a field clashes with another name; every
//! clash case between fields, parameters and globals
use crate::is;
use warp::normalize::{capture_hints, clear_shown_hints};
use warp::wasm_emitter::eval;

fn hint_reasons(code: &str) -> (String, Vec<String>) {
	clear_shown_hints();
	let (value, hints) = capture_hints(|| eval(code));
	(value.serialize(), hints.into_iter().map(|hint| hint.reason).collect())
}

#[test]
fn a_bare_field_reads_the_receiver() {
	is!("class P { x: int; def double() := x * 2 }\nP(3).double()", 6);
	is!("class P { x: int; def inc() { x = x + 1 } }\np = P(3)\np.inc()\np.x", 4);
}

#[test]
fn self_or_this_names_the_field_a_parameter_shadows() {
	is!("class P { x: int; def setx(x) { self.x = x } }\np = P(3)\np.setx(5)\np.x", 5);
	is!("class P { x: int; def setx(x) { this.x = x } }\np = P(3)\np.setx(5)\np.x", 5);
}

#[test]
fn an_explicit_self_parameter_reads_the_field_either_way() {
	is!("class P { x: int; def more(self, n) { self.x + n } }\nP(3).more(4)", 7);
	is!("class P { x: int; def more(self, n) { x + n } }\nP(3).more(4)", 7);
}

#[test]
fn a_parameter_assigned_to_itself_hints_the_field() {
	let (value, reasons) = hint_reasons("class P { x: int; def setx(x) { x = x } }\np = P(3)\np.setx(5)\np.x");
	assert_eq!(value, "3");
	assert!(reasons.iter().any(|reason| reason.contains("the field is self.x")), "{reasons:?}");
}

#[test]
fn a_field_shadowing_a_global_reads_the_field() {
	is!("x = 100\nclass P { x: int; def get() := x }\nP(3).get()", 3);
}

#[test]
fn a_method_changing_its_object_may_be_named_like_a_library_word() {
	is!("class Q { x: int; def get() { x = 7; x } }\nq = Q(3)\n[q.get(), q.x]", warp::ints(vec![7, 7]));
	is!("class Q { x: int; def contains() { x = 8; x } }\nq = Q(3)\n[q.contains(), q.x]", warp::ints(vec![8, 8]));
}
