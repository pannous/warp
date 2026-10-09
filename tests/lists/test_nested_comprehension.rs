// card double-for-comprehension: a list comprehension with several `for` clauses nests them in order, as Python does;
// a filter `if c` sits between or after them. It was silently empty
use crate::is;
use warp::warp_parser::parse;

#[test]
fn two_for_clauses_nest() {
	is!("[f * i for f in [1, 2] for i in 0..3]", parse("[0 1 2 0 2 4]"));
	is!("count([f * i for f in [1, 2] for i in 0..3])", 6);
	is!("[x + y for x in [10, 20] for y in [1, 2] for z in [0, 0]]", parse("[11 11 12 12 21 21 22 22]"));
}

#[test]
fn filters_sit_between_and_after_the_clauses() {
	is!("[f * i for f in [1, 2] for i in 0..3 if i > 0]", parse("[1 2 2 4]"));
	is!("[f * i for f in [1, 2] if f > 1 for i in 0..3]", parse("[0 2 4]"));
	is!("[f + i for f in [1, 2, 3] if f != 2 for i in [10, 20] if i < 20]", parse("[11 13]"));
}
