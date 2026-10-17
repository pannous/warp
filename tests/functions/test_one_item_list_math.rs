// card sin-scalar: a math word over a list of one number gives a list of one, as over longer lists; the compile-time
// evaluation of exact reals read `[4]` as 4
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_list_of_one_number_stays_a_list() {
	assert_eq!(printed("sqrt [4]"), "[2]");
	assert_eq!(printed("exp [0.0]"), "[1]");
	assert_eq!(printed("sin([0.0])"), "[0]");
	assert_eq!(printed("xs = [4]; sqrt xs"), "[2]");
}

#[test]
fn one_number_stays_one_number() {
	assert_eq!(printed("sqrt 4"), "2");
	assert_eq!(printed("x = √2 * √2; x"), "2");
}
