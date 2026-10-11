//! Advice whose preferred form can replace some text offers it as an "I meant" fix (card advise-fix): the playground
//! shows it as a button like a hint's; advice that names no replacement (`on … before the loop`) stays without one
use warp::fixits::{fixed, Fix};
use warp::normalize::{capture_hints, CapturedHint};
use warp::wasm_emitter::eval;

fn advice(code: &str, preferred: &str) -> CapturedHint {
	let (_, hints) = capture_hints(|| eval(code));
	hints.into_iter().find(|hint| hint.canonical == preferred).unwrap_or_else(|| panic!("no advice {preferred} for {code}"))
}

/// The advice preferring `preferred` in `code` offers a fix, which gives `expected`
fn assert_advice_fix(code: &str, preferred: &str, expected: &str) {
	let advice = advice(code, preferred);
	let (line, column) = advice.line_and_column();
	let fix: Fix = advice.fix().expect("advice with a replacement offers it");
	let source = fixed(code, line, column, &fix).expect("the advice's text is in the source");
	assert_eq!(eval(&source).serialize(), expected, "{source}");
}

#[test]
fn a_block_assignment_offers_global_at_main() {
	assert_advice_fix("x=1; inc:={x=x+1}; do inc; x", "global x", "2");
}

#[test]
fn then_after_a_comparison_offers_the_pipe() {
	assert_advice_fix("yes_no(b) := if b then \"y\" else \"n\"; 1 < 2 then yes_no", "(1<2) |> yes_no", "'y'");
}

#[test]
fn advice_without_a_replacement_offers_no_fix() {
	assert!(advice("x=1; inc:={x=x+1}; do inc; x", "x = inc()").fix().is_none());
}
