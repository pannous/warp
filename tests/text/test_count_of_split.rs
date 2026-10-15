//! card count-split: count takes the call after it, as sum does: count(split t) counts the parts
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn count_of_a_split_counts_the_parts() {
	assert_eq!(printed("count(split \"a b c\")"), "3");
	assert_eq!(printed("x = \"a b\"; count(split x)"), "2");
	assert_eq!(printed("count(sorted [3 1])"), "2");
}

#[test]
fn count_of_an_item_in_a_list_stays() {
	assert_eq!(printed("xs = [1 2 1]; count(xs, 1)"), "2");
	assert_eq!(printed("count(\"abab\", \"a\")"), "2");
}
