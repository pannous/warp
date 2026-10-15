//! card count-sorted: sort and sorted take the braceless call inside their parentheses, as count does: sort(split x)
//! sorts the parts
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn sort_of_a_braceless_call_sorts_its_result() {
	assert_eq!(printed("x = \"b a\"; sorted(split x)"), "['a' 'b']");
	assert_eq!(printed("x = \"b a\"; sort(split x)"), "['a' 'b']");
	assert_eq!(printed("xs = [3 1 2]; sort(reverse xs)"), "[1 2 3]");
	assert_eq!(printed("x = \"a b\"; count(sorted(split x))"), "2");
}

#[test]
fn a_word_of_one_argument_takes_the_library_call_inside_its_parentheses() {
	assert_eq!(printed("xs = [3 1 2]; first(reverse xs)"), "2");
	assert_eq!(printed("xs = [3 1 2]; count(reverse xs)"), "3");
	assert_eq!(printed("xs = [3 1 2]; count reverse xs"), "3");
	assert_eq!(printed("xs = [3 1 2 1]; count(xs, 1)"), "2");
}
