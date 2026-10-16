//! cards infix-map, map-dot: an iteration word between a list and its function, `xs map f`, is `xs.map(f)`, and
//! `.a` after it the field-read function (the user's `methods(math) map .doc`)
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn an_iteration_word_between_list_and_function() {
	assert_eq!(printed("xs = [1 2]; xs map (x => x*2)"), "[2 4]");
	assert_eq!(printed("xs = [1 2]; xs map {it*2}"), "[2 4]");
	assert_eq!(printed("square x := x*x; [1 2] map square"), "[1 4]");
	assert_eq!(printed("[1 2 3 4] filter {it > 2}"), "[3 4]");
	assert_eq!(printed("xs = [1 2]; ys = xs map {it+1}; ys"), "[2 3]");
}

#[test]
fn a_dot_name_after_an_iteration_word_reads_the_field() {
	assert_eq!(printed("ps = [{a:1} {a:2}]; ps map .a"), "[1 2]");
	assert_eq!(printed("ps = [{a:1} {a:2}]; ps.map(.a)"), "[1 2]");
}

#[test]
fn prefix_iteration_stays() {
	assert_eq!(printed("map [1 2] {it*2}"), "[2 4]");
	assert_eq!(printed("map {it*2} [1 2]"), "[2 4]");
}

#[test]
fn a_calling_word_before_a_prefix_iteration_stays() {
	assert_eq!(printed("f(xs) := { return map {it*2} xs }; f([1 2])"), "[2 4]");
}
