//! The Fail effect (card fail-effect, wiki/effects.md): a function that can raise an error says so in `effects of f`;
//! `try` handles it, `! Pure` refuses it, and a failing function still folds like a pure one. The rule: it raises
//! (raise, throw, fail, panic, error "…"), divides by what may be 0 (/ // div % rem), or calls a library word or crate
//! that answers with an Error, indexes (an index out of range, a missing key) or casts to a number (card
//! fail-effect-indexing)
use crate::common::fails_with;
use crate::is;
use warp::analyzer::effects::{effects_of, Effect::*, EffectSet};

fn fails(code: &str, function: &str) -> bool {
	effects_of(code, function).is_some_and(|effects| effects.contains(Fail))
}

#[test]
fn raising_and_dividing_by_a_variable_fail() {
	assert!(fails("f(x) := 10/x", "f"));
	assert!(fails("f(x) := x % x", "f"));
	assert!(fails("f(x, y) := x div y", "f"));
	assert!(!fails("f(x) := x div 2", "f"));
	assert!(fails("g(x) := raise \"no\"", "g"));
	assert!(fails("g(x) := throw \"no\"", "g"));
	assert!(fails("h(x) := if x < 0 then error \"neg\" else x", "h"));
	assert!(fails("f(x) := 10/x\nk(x) := f(x) + 1", "k"));
}

#[test]
fn dividing_by_a_number_and_try_do_not_fail() {
	assert_eq!(effects_of("half(x) := x/2", "half"), Some(EffectSet::PURE));
	assert_eq!(effects_of("f(x) := try 10/x else 0", "f"), Some(EffectSet::PURE));
	assert_eq!(effects_of("f(x) := 10/x\nk(x) := try f(x) else 0", "k"), Some(EffectSet::PURE));
	assert!(fails("f(x) := 10/x\nk(x) := (try f(x) else 0) + f(x)", "k"));
}

#[test]
fn effects_of_names_fail() {
	is!("f(x) := 10/x\neffects of f", warp::Node::Symbol("Fail".into()));
	let answer = warp::wasm_emitter::eval("g(x) := { puts x; raise \"no\" }\neffects of g").serialize();
	assert_eq!(answer, "(IO Fail)");
}

#[test] // lib/json.warp's parse_json calls the host word std_pure, whose failure is raised in the program
fn a_library_word_that_answers_with_an_error_fails() {
	is!("use json\nf(s) := parse_json(s)\neffects of f", warp::Node::Symbol("Fail".into()));
	let answer = warp::wasm_emitter::eval("use json\nf(s) := try parse_json(s) else 0\neffects of f").serialize();
	assert!(!answer.contains("Fail"), "{answer}");
}

#[test]
fn pure_refuses_fail_but_folding_does_not() {
	fails_with("f(x) := 10/x ! Pure\nf(2)", "f is declared ! Pure but performs Fail");
	is!("f(x) := 10/x ! Fail\nf(2)", 5);
	is!("f(x) := (try 10/x else 0) ! Pure\nf(0)", 0);
	is!("f(x) := 10/x\nk(x) := (try f(x) else 0) ! Pure\nk(0)", 0);
	is!("f(x) := 10/x\nf(2)", 5);
}

#[test] // card fail-effect-indexing: an index out of range, a missing key, a failed cast
fn indexing_a_key_and_a_number_cast_fail() {
	assert!(fails("f(xs) := xs#5", "f"));
	assert!(fails("f(xs, i) := xs[i]", "f"));
	let field = warp::wasm_emitter::eval("f(q) := q.k\neffects of f").serialize();
	assert!(field.contains("Fail"), "{field}"); // q.k of a map is q#("k"+1) once lowered
	assert!(fails("f(x) := x as int", "f"));
	assert!(!fails("first(x) := [x, x]#1", "first"));
	assert!(!fails("f(xs) := count xs", "f"));
	assert!(!fails("f(x) := x as text", "f"));
	assert!(!fails("f(xs) := try xs#5 else 0", "f"));
}

#[test] // cards print-effects, print-effects-walked: the effects print as one value, not one letter per line
#[cfg(feature = "native")]
fn print_effects_of_prints_the_effects() {
	assert_eq!(crate::common::printed("f(x) := 10/x\nprint effects of f").trim(), "Fail");
	assert_eq!(crate::common::printed("g(x) := { puts x; raise \"no\" }\nprint effects of g").trim(), "(IO Fail)");
}
