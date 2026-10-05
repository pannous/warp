// Unknown words (user, 2026-10-05, refining P62/P63): a word alone is a symbol, with a near-miss warning when it is close to
// a defined name; a word applied to an argument is an error, brackets change nothing; `data …` is the data prefix
use crate::common::fails_with;
use warp::diagnostic::take_warnings;
use warp::wasm_emitter::eval;

fn warns_near_miss(code: &str) -> bool {
	take_warnings();
	eval(code);
	take_warnings().iter().any(|warning| warning.message.contains("did you mean"))
}

#[test]
fn test_a_lone_word_is_a_symbol() {
	assert_eq!(eval("red").serialize().trim(), "red");
	assert_eq!(eval("[red green]").serialize().trim(), "[red green]");
}

#[test]
fn test_a_word_close_to_a_name_warns() {
	assert!(warns_near_miss("pirnt"));
	assert!(warns_near_miss("counter=1; countr"));
	assert!(!warns_near_miss("red"));
	assert!(!warns_near_miss("counter=1; counter"));
}

#[test]
fn test_a_word_applied_to_an_argument_is_an_error() {
	fails_with("x=2; cube x", "undefined: cube");
	fails_with("[cube 3]", "undefined: cube");
}

#[test]
fn test_data_is_the_data_prefix() {
	assert_eq!(eval("data cube 3").serialize().trim(), "cube 3");
	fails_with("quote cube 3", "undefined: quote");
}
