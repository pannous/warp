//! card dot-lambda: a leading-dot word where a function is expected is the lambda on its member, as Swift's `.member`
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

const PEOPLE: &str = "people = [{name: \"Ann\" age: 3} {name: \"Bob\" age: 1}]; ";

#[test]
fn a_dot_word_reads_the_member_of_each_item() {
	assert_eq!(printed(&format!("{PEOPLE}people.sort(.age)")), "[{name:\"Bob\" age:1} {name:\"Ann\" age:3}]");
	assert_eq!(printed(&format!("{PEOPLE}people.map(.name)")), "[\"Ann\" \"Bob\"]");
	assert_eq!(printed("map([\"ab\"], .upper)"), "[\"AB\"]");
}

#[test]
fn a_dot_word_applies_a_number_word() {
	assert_eq!(printed("[1.5 2.7].map(.floor)"), "[1 2]");
	assert_eq!(printed("[-1 2].map(.abs)"), "[1 2]");
}

/// the number word's method form on a parameter: its field if it is an object, else the word
#[test]
fn a_parameter_s_number_word_method_is_the_word() {
	assert_eq!(printed("f(x) := x.floor; f(2.5)"), "2");
	assert_eq!(printed("[4].map(x => x.sqrt)"), "[2]");
	assert_eq!(printed("f(p) := p.floor; f({floor: 7})"), "7");
}

#[test]
fn a_leading_dot_number_stays_a_number() {
	assert_eq!(printed("xs = [.5 1]; xs"), "[0.5 1]");
}

/// `.m` is `x => x.m` through every pass: a library word read as a member, a dot word assigned as a value
#[test]
fn a_dot_word_is_the_written_lambda() {
	assert_eq!(printed("[[1 2] [3]].map(.first)"), "[1 3]");
	assert_eq!(printed("f = .name; f({name: 1})"), "1");
}
