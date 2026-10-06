// Two print statements on one line separated only by spaces are an error naming the separators (user decision 2026-10-03)
use crate::common::fails_with;
use crate::is;

const HINT: &str = "two statements on one line? separate them with `;` or a newline";

#[test]
fn two_prints_on_one_line_need_a_separator() {
	fails_with("g=\"hi\"; print g    print g", HINT);
	fails_with("print \"a\" print \"b\"", HINT);
	fails_with("g=\"hi\"; print g*2    print(g, g)   print g, g", HINT);
}

#[test]
fn separated_prints_still_run() {
	is!("g=\"hi\"; print g;    print g; g", "hi"); // print gives nothing (issue #18)
	is!("print \"a\"\nprint \"b\"", warp::Node::Empty);
}
