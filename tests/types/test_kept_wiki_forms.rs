use crate::is;

// The wiki definition forms that stay after the phrase cleanup (type-name parameters `fib int i` went).
#[test]
fn the_kept_wiki_definition_forms() {
	is!("fibonacci number = if number<2 : number else fibonacci(number-1) + fibonacci(number-2); fibonacci 10", 55);
	is!("square of a number = it*it; square 3", 9);
	is!("square of a number = it*it; square of 4", 16);
	is!("to square a number: it*it; square 3", 9);
	is!("foo of int = it+it; foo 3", 6);
}
