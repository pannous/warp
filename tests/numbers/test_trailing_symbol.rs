// card trailing-symbol: a word after an assignment's value is never dropped silently: σ joins the spread as a unit
// word does (`5 ± 1 σ` is `5 ± 1σ`), an unknown word is an error
use crate::common::fails_with;
use warp::wasm_emitter::eval;

#[test]
fn a_spaced_sigma_makes_a_gaussian() {
	assert_eq!(eval("x = 5 ± 1 σ; str(x + x)"), "10.0 ± 2.0σ");
	assert_eq!(eval("x = 5 ± 1 σ; y = 2 ± 1 σ; str(x + y)"), "7.0 ± 1.4σ");
}

#[test]
fn an_unknown_word_after_an_assignment_is_an_error() {
	fails_with("y = 1 zork; y", "zork");
	fails_with("y = 1 zork", "zork");
	fails_with("x = 5 ± 1 zork; x", "zork");
}

#[test]
fn a_known_word_after_an_assignment_is_an_error() {
	fails_with("x = 3; y = 1 x; y", "does nothing");
	fails_with("y = 1 print; y", "does nothing");
	assert_eq!(eval("y = 2 m; str(y)"), "2m");
}
