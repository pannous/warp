//! Every spelling of a type-name definition is one parse (notes/matching.md, wiki_features row 25): `=` and `:=`,
//! a `to` phrase with a `:` or a `{…}` body. square is defined in each test (no built-in square, P39).
use crate::is;

#[test]
fn test_define_operator_takes_the_same_slots_as_assign() {
	is!("square of a number := it*it; square 3", 9);
	is!("square a number := number*number; square 3", 9);
	is!("square of number := it*it; square 4", 16);
	is!("fib int i := if i<2 : i else fib(i-1)+fib(i-2); fib 10", 55);
}

#[test]
fn test_to_phrase_with_a_block_body() {
	is!("to square a number { it*it }; square 3", 9);
	is!("to square a number {\n  return number*number\n}\nsquare 5", 25);
	is!("to add number a to number b { a+b }; add(1, 2)", 3);
}

#[test]
fn test_wiki_forms() {
	is!("fibonacci number = if number<2 : 0 else fibonacci(number-1) + fibonacci(number-2); fibonacci 10", 0); // the wiki's base case is 0
	is!("fibonacci number = if number<2 : number else fibonacci(number-1) + fibonacci(number-2); fibonacci 10", 55);
	is!("square of a number = it*it; square 3", 9);
	is!("to square a number: it*it; square 3", 9);
	is!("to square a number: return it * it; square 3", 9);
	is!("foo of int = it+it; foo 3", 6);
	is!("fib int i = if i<2 : i else fib(i-1)+fib(i-2); fib 10", 55);
}
