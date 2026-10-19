// card why-sqrt: √ ∛ − ‖‖ of an exact real, a float and an approximation go through the one function table of
// real.rs, as sin/cos/ln do: no operator is special
use warp::wasm_emitter::eval;

#[test]
fn prefix_words_of_exact_reals_stay_exact() {
	assert_eq!(eval("sqrt(2)*sqrt(2)"), eval("2"));
	assert_eq!(eval("cbrt(2)*cbrt(2)*cbrt(2)"), eval("2"));
	assert_eq!(eval("abs(-sqrt(2))*abs(-sqrt(2))"), eval("2"));
	assert_eq!(eval("-(-sqrt(2)) == sqrt(2)"), eval("true"));
	assert_eq!(eval("sqrt(8)"), eval("2*sqrt(2)"));
}

#[test]
fn prefix_words_keep_floats_and_approximations() {
	assert_eq!(eval("abs(-sin(1.0))"), eval("sin(1.0)"));
	assert_eq!(eval("-sqrt(2.5) + sqrt(2.5)"), eval("0.0"));
}
