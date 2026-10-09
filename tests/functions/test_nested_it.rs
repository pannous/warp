//! `it` belongs to the innermost lambda (Kotlin): a nested `{ it … }` shadows the outer `it`, and a function's `it`
//! binding stops at a lambda inside its body (cards nested-it, it-leak; notes/implicit_params.md)
use crate::is;
use warp::warp_parser::parse;

#[test]
fn test_glued_block_after_a_method_is_a_lambda() {
	is!("[1,2].map{ it*2 }", parse("[2 4]"));
	is!("xs=[1,2,3]; xs.filter{ it > 1 }", parse("[2 3]"));
}

#[test]
fn test_trailing_block_of_an_assigned_method_call() {
	is!("f(xs) := xs.map { it*10 }; f([1,2])", parse("[10 20]"));
	is!("ys = [1,2].map { it*10 }; ys", parse("[10 20]"));
}

#[test]
fn test_inner_it_shadows_the_outer_it() {
	is!("[[1,2],[3]].map{ it.map{ it*10 } }", parse("[[10 20] [30]]"));
	is!("[[1,2],[3]].map { it.map { it*10 } }", parse("[[10 20] [30]]"));
}

#[test]
fn test_inner_dollar_zero_shadows_the_outer() {
	is!("[[1,2],[3]].map{ $0.map{ $0*10 } }", parse("[[10 20] [30]]"));
}

#[test]
fn test_function_it_stops_at_an_inner_lambda() {
	is!("f(x) := { [1,2].map{ it + x } }; f(10)", parse("[11 12]"));
	is!("f(x) := [1,2].map { it + x }; f(10)", parse("[11 12]"));
}

#[test]
fn test_trailing_block_of_a_lambda_body_method() {
	is!("[[1,2],[3]].map { x -> x.map { it*10 } }", parse("[[10 20] [30]]"));
	is!("f = x => x.map { it*10 }; f([1,2])", parse("[10 20]"));
}

// A lambda naming its parameter leaves `it` to the function around it (card lambda-def)
#[test]
fn test_named_lambda_parameter_leaves_the_outer_it() {
	is!("scale := [1 2].map(x => x * it); scale 3", parse("[3 6]"));
	is!("scale(it) := [1 2].map(x => x * it); scale(3)", parse("[3 6]"));
}
