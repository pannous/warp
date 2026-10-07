//! `xs where it > 1` keeps the elements the condition holds for (card xs-where: it used to do nothing silently)
use crate::is;
use warp::wasm_emitter::eval;
use warp::{parse, Node};

#[test]
fn where_filters_by_it() {
	is!("xs = [1, 2, 3]; xs where it > 1", parse("[2 3]"));
	is!("xs = [\"a\", \"\"]; count(xs where it != \"\")", 1);
	is!("xs = [1, 2, 3, 4, 5]; xs where it > 1 and it < 5", parse("[2 3 4]"));
	is!("xs = [1, 2, 3]; ys = xs where it > 1; count(ys)", 2);
	is!("f(v) := v where it > 1; f([1, 2, 3])", parse("[2 3]"));
	is!("people = [{age: 20}, {age: 10}]; count(people where it.age > 18)", 1);
}

#[test]
fn a_where_condition_without_it_is_an_error() {
	let Node::Error(message) = eval("xs = [1, 2, 3]; xs where x > 1") else { panic!("should be an error") };
	assert!(format!("{message}").contains("where it > 1"), "{message}");
}

/// Haskell's binding `e where x = 3`: the assignment runs first, then e (not a filter)
#[test]
fn where_with_an_assignment_binds() {
	is!("x * 2 where x = 3", 6);
	is!("xs.reduce(0) { $0 + $1 } where xs = [1,2,3]", 6);
}
