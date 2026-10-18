//! A grouped first value stays inside its surrounding comma tuple (card parenthesized-first).
use warp::wasm_emitter::eval;

#[test]
fn parenthesized_first_value_stays_in_the_tuple() {
	assert_eq!(eval("xs = ((try 1/0 else 0), 2); xs").serialize(), "(0 2)");
	assert_eq!(eval("xs = ((if yes then 1 else 0), 2); xs").serialize(), "(1 2)");
	assert_eq!(eval("xs = ((1 == 1), 2); xs").serialize(), "(yes 2)");
}
