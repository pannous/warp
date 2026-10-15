//! card cos-instead: the math words of one number broadcast over a list as floor does, `cos([0, 1])` is
//! `[cos(0) cos(1)]`, not cos of the last item; a prefix operator over a parenthesized list too, `abs([-1, 2])`
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn math_words_map_over_a_list() {
	assert_eq!(printed("cos([0, 1])").split_whitespace().count(), 2);
	assert_eq!(printed("cos [0 0]"), "[1 1]");
	assert_eq!(printed("exp([0, 0, 0])"), "[1 1 1]");
	assert_eq!(printed("sin(xs) where xs = [0 0]"), "[0 0]");
	assert_eq!(printed("xs = [0, 0]; cos(xs)"), "[1 1]");
	assert_eq!(printed("log [1 1]"), "[0 0]");
}

#[test]
fn prefix_operators_map_over_a_parenthesized_list() {
	assert_eq!(printed("abs([-1, 2])"), "[1 2]");
	assert_eq!(printed("sqrt([4 9])"), "[2 3]");
}

#[test]
fn one_number_stays_one() {
	assert_eq!(printed("cos(0)"), "1");
	assert_eq!(printed("abs(-3)"), "3");
}
