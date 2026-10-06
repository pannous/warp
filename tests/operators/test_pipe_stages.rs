// wiki/pipe.md `2|square|root`, `square 2|root`: a pipe into a function defined in the spaced form (`square x := x*x`,
// rewritten only after the pipe pass) and into a word operator alone (`sqrt`, `cbrt`, `abs`)
use warp::ints;
use crate::is;

#[test]
fn a_pipe_into_a_spaced_definition() {
	is!("square x := x*x; 2|square", 4);
	is!("square x := x*x; [1,2]|square", ints(vec![1, 4]));
	is!("square x := x*x; square 2|square", 16);
}

#[test]
fn a_pipe_into_a_word_operator() {
	is!("4|sqrt", 2);
	is!("27 | cbrt", 3);
	is!("-4|abs", 4);
	is!("square x := x*x; 2|square|sqrt", 2);
	is!("square x := x*x; square 2|sqrt", 2);
}
