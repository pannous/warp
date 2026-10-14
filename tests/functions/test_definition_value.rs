// A program that only defines a function has the value ø (it used to fail WASM validation or call the head)
use crate::is;
use warp::*;

#[test]
fn a_definition_alone_is_empty() {
	is!("square := it*it", Empty);
	is!("f(x) := x + 1", Empty);
	is!("square = {it*it}", Empty);
	is!("square := it*it; square 4", 16);
}

#[test]
fn it_names_the_one_parameter_of_a_definition() {
	// wiki/Home.md, declaration.md, syntax.md: `fibonacci it - 2` inside `fibonacci number := …`
	is!("fibonacci number := if number<2 : 1 else fibonacci(number - 1) + fibonacci it - 2; fibonacci(10)", 89);
	is!("twice x := it * 2; twice 4", 8);
	is!("f(x) := x + it; f(3)", 6);
}

#[test]
fn a_spaced_definition_names_its_parameters() {
	// wiki/examples.md `square x:=x²`, wiki/symbol.md `x y z := y*y+u`
	is!("g n := n + 1; g 4", 5);
	is!("square x := x * x; square 5", 25);
	is!("x y z := y*y+z; x(2, 3)", 7);
}
