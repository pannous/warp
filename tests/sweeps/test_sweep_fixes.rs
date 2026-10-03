//! Small bugs found by the ignored-test sweep: a lone `false`, `do add x to list`, `not` of an object, a call statement in a block body

use warp::is;

#[test]
fn a_lone_false_is_zero() {
	is!("false", 0);
	is!("false;", 0);
	is!("(false)", 0);
}

#[test]
fn do_runs_an_add_to_phrase() {
	is!("pixel=[1 2 3]; do add 4 to pixel; pixel", warp::ints(vec![1, 2, 3, 4]));
}

#[test]
fn a_builtin_call_statement_runs_inside_a_loop_body() {
	is!("x=0; while x<2 {puti x; x++}; x", 2);
	is!("for i in 1 to 3 {puti i}; i", 4);
}
