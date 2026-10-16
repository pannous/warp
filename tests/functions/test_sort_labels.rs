//! card sort-xs: a sort label and the function after it, written without the colon or parentheses, are the labeled
//! function of a call, `sort xs by >` is `sort(xs, by: >)`, in prefix and infix form
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_sort_label_before_an_operator() {
	assert_eq!(printed("xs=[3 1 2]; sort xs by >"), "[3 2 1]");
	assert_eq!(printed("xs=[3 1 2]; xs sort by >"), "[3 2 1]");
	assert_eq!(printed("xs=[3 1 2]; sort xs by: >"), "[3 2 1]");
	assert_eq!(printed("xs=[3 1 2]; xs sort by: >"), "[3 2 1]");
}

#[test]
fn a_sort_label_before_a_field_or_block() {
	assert_eq!(printed("ps=[{a:2} {a:1}]; sort ps by {it.a}"), "[a:1 a:2]");
	assert_eq!(printed("ps=[{a:2} {a:1}]; sort ps by .a"), "[a:1 a:2]");
	assert_eq!(printed("ps=[{a:2} {a:1}]; sort ps by: .a"), "[a:1 a:2]");
	assert_eq!(printed("ps=[{a:2} {a:1}]; ps sort by .a"), "[a:1 a:2]");
}

/// reviewer: the parser hangs the phrase on the dangling operator in a value position and the rest of a chain on it
#[test]
fn a_sort_label_before_an_operator_in_a_value_or_chain() {
	assert_eq!(printed("xs=[3 1 2]; ys = sort xs by >; ys"), "[3 2 1]");
	assert_eq!(printed("def f(xs):=sort xs by >; f([3 1 2])"), "[3 2 1]");
	assert_eq!(printed("xs=[3 1 2]; xs sort by > map {it*10}"), "[30 20 10]");
	assert_eq!(printed("ps=[{a:2} {a:1}]; ps sort by .a map .a"), "[1 2]");
}

/// reviewer: a prefix sort before a chain ends at its labeled function, `(sort xs by >) map f`
#[test]
fn a_prefix_sort_before_a_chain() {
	assert_eq!(printed("xs=[3 1 2]; sort xs by > map {it*10}"), "[30 20 10]");
	assert_eq!(printed("xs=[3 1 2]; sort xs by: > map {it*10}"), "[30 20 10]");
	assert_eq!(printed("ps=[{a:2} {a:1}]; sort ps by .a map .a"), "[1 2]");
	assert_eq!(printed("xs=[3 1 2]; (sort xs by: >)"), "[3 2 1]");
	assert_eq!(printed("xs=[3 1 2]; (sort xs by >) map {it*10}"), "[30 20 10]");
}

#[test]
fn a_word_named_like_a_label_elsewhere_stays() {
	assert_eq!(printed("by=2; by > 1"), "yes");
	assert_eq!(printed("xs=[3 1 2]; sort xs"), "[1 2 3]");
}
