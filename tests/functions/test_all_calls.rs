// `f all xs` (wiki/all.md): `all` splices the list into its items, f applies to each: `square all xs` is
// `[square(1) square(2) square(3)]`, the same as broadcasting `square xs` (wiki/broadcasting.md)
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_function_applied_to_all_items() {
	assert_eq!(printed("square(x) := x*x; xs = [1, 2, 3]; square all xs"), "[1 4 9]");
	assert_eq!(printed("square(x) := x*x; square all [1, 2, 3]"), "[1 4 9]");
	assert_eq!(printed("def twice(x){ x * 2 }; ys = twice all [1, 2]; ys"), "[2 4]");
	assert_eq!(printed("upper all [\"ab\", \"c\"]"), "[\"AB\" \"C\"]"); // a library word
}

#[test]
fn broadcasting_compares_as_a_list() {
	assert_eq!(printed("square(x) := x*x; square [1 2 3] == [1 4 9]"), "1");
	assert_eq!(printed("square(x) := x*x; (square [1 2 3]) == [1 4 9]"), "1");
}
