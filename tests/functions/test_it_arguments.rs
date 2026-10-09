//! An expression of `it` where a function is expected is the function of `it` (card map-it, notes/welcoming.md):
//! `xs.map(it * 2)` like `xs.map { it * 2 }`, for iteration words and every function-taking parameter
use crate::is;
use warp::warp_parser::parse;

#[test]
fn an_it_argument_of_an_iteration_word_is_a_function() {
	is!("(1…3).map(it * 2)", parse("[2 4 6]"));
	is!("[1 2 3 4].filter(it > 2)", parse("[3 4]"));
	is!("map [1 2] (it * 10)", parse("[10 20]"));
	is!("[3 1 2].sort(by: -it)", parse("[3 2 1]"));
	is!("[1 2 3].find(it > 1)", 2);
}

#[test]
fn an_it_argument_of_a_defined_function_is_a_function() {
	is!("apply(f, x) := f(f(x)); apply(it + 3, 1)", 7);
}

#[test]
fn an_it_argument_reads_the_variables_around_it() {
	is!("k = 5; [1 2].map(it + k)", parse("[6 7]"));
}

#[test]
fn the_it_of_the_argument_is_not_the_parameter_of_the_function_around_it() {
	is!("doubled(xs) := xs.map(it * 2); doubled [1 2]", parse("[2 4]"));
	is!("big(xs) := filter(xs, it > 1); big [1 2 3]", parse("[2 3]"));
}
