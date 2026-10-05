//! wiki/function-pointer.md: `map square on [1 2 3]`, `map function square on …`, `map &square …` with the function
//! first, for a function defined either way (`square:=it*it` makes the parser apply it, `map (square (on xs))`)
use warp::is;
use warp::wasp_parser::parse;

#[test]
fn test_map_a_function_on_a_list() {
	is!("square:=it*it; map square on [1 2 3]", parse("[1 4 9]"));
	is!("def square(x){x*x}; map square on [1 2 3]", parse("[1 4 9]"));
	is!("xs=[1 2 3]; square:=it*it; map square on xs", parse("[1 4 9]"));
}

#[test]
fn test_map_a_function_reference_on_a_list() {
	is!("square:=it*it; map function square on [1 2 3]", parse("[1 4 9]"));
	is!("square:=it*it; map &square on [1 2 3]", parse("[1 4 9]"));
}

#[test]
fn test_map_a_function_first_without_on() {
	is!("square:=it*it; map square [1 2 3]", parse("[1 4 9]"));
	is!("square:=it*it; map &square [1 2 3]", parse("[1 4 9]"));
}
