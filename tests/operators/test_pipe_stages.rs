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

// card pipe-compose: wiki/pipe.md `printSum := sum|print`: a pipe between functions is their composition, the function
// that calls them one after the other (`(f|g)(x)` is `g(f(x))`)
#[test]
fn a_pipe_between_functions_composes_them() {
	is!("rootSum := sum|sqrt; rootSum 1 3 5", 3); // printSum := sum|print prints 6, its value is print's ø
	is!("square x := x*x; f := square|sqrt; f 3", 3);
	is!("square x := x*x; f := square|square|sqrt; f(3)", 9);
	is!("def twice(x) = 2*x; f = twice|abs; f(-4)", 8);
}

// card pipe-applied: wiki/pipe.md `f|g x := g(f(x))`: the composition applied to the argument after it
#[test]
fn a_composition_takes_the_argument_after_it() {
	is!("square x := x*x; sum|square 1 2", 9); // sum|print 1 2 3 prints 6
	is!("square x := x*x; sum|square [1 2]", 9);
	is!("square x := x*x; square|sqrt 3", 3);
	is!("square x := x*x; square|square|sqrt 3", 9);
	is!("def twice(x) = 2*x; twice|abs -4", 8);
}
