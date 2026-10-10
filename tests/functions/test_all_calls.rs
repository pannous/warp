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
	// P143 (user): a parameter typed as no truth value takes the call, `(square …) == …`
	assert_eq!(printed("square number = number*number; square [1 2 3] == [1 4 9]"), "yes");
	assert_eq!(printed("def square(x: int){ x*x }; square 3 == 9"), "yes");
	assert_eq!(printed("square(x) := x*x; (square [1 2 3]) == [1 4 9]"), "yes");
}

#[test]
fn an_untyped_call_next_to_a_comparison_is_ambiguous() {
	// P143 (user): both readings type-check, so the error names both forms; P149 (user): only when the body accepts
	// anything, `x*x` needs a number
	crate::common::fails_with("same(x) := x; same 3 == 9", "write (same 3) == 9 or same(3 == 9)");
	assert_eq!(printed("square(x) := x*x; square 3 == 9"), "yes");
}

#[test]
fn all_over_nested_lists_broadcasts_again() {
	assert_eq!(printed("square(x) := x*x; square all [[1, 2], [3]]"), "[[1 4] [9]]");
}

#[test]
fn a_list_built_by_appending_broadcasts() {
	assert_eq!(printed("square(x) := x*x; xs = []; xs.add(2); xs.add(3); ys = square all xs; ys"), "[4 9]");
	assert_eq!(printed("square(x) := x*x; xs = []; for i in 1..3 { xs = xs + [i] }; square xs"), "[1 4]");
}

#[test]
fn every_definition_form_reads_a_comparison_alike() {
	// card p143-misses: `def square(x) = x*x` gave 0 silently; P149 (user): a body in arithmetic takes the call
	for code in ["def square(x) = x*x; square 3 == 9", "square := it*it; square 3 == 9", "square = x => x*x; square 3 == 9", "def f(x){ x + 1 }; f 1 < 5"] {
		assert_eq!(printed(code), "yes", "{code}");
	}
	for code in ["def same(x) = x; same 3 == 9", "same := it; same 3 == 9", "same = x => x; same 3 == 9"] {
		crate::common::fails_with(code, "write (same 3) == 9 or same(3 == 9)");
	}
}

#[test] // wiki/plural.md: `each friend` walks the list friends, as `all friends` does; `f of x` calls a defined function (card plural-names)
fn a_singular_name_walks_its_plural_list() {
	assert_eq!(printed("def sq(x){x*x}; xs=[1 2 3]; sq each x"), "[1 4 9]");
	assert_eq!(printed("def sq(x){x*x}; cities=[1 2]; sq each city"), "[1 4]");
	assert_eq!(printed("def sq(x){x*x}; xs=[1 2]; sq each xs"), "[1 4]");
	assert_eq!(printed("def sq(x){x*x}; sq of 3"), "9");
	assert_eq!(printed("use text; text=\"hi you\"; words of text"), "[\"hi\" \"you\"]");
}

#[test]
#[cfg(feature = "native")]
fn print_each_friend() {
	assert!(crate::common::printed("friends=[james, peter]; print each friend").starts_with("james\npeter\n"));
}
