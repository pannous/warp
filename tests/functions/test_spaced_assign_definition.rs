// A spaced definition written with `=` (wiki/Home.md, wiki/=.md): `fibonacci number = … fibonacci …` defines the
// function when its body calls it; `print x = 5` stays what it was
use crate::is;

#[test]
fn a_recursive_spaced_definition_with_equals_defines_the_function() {
	is!("fibonacci number = if number<2 : 1 else fibonacci(number - 1) + fibonacci it - 2\nfibonacci 10", 89);
	is!("fib int i = if i<2 : 1 else fib(i - 1) + fib i - 2\nfib 5", 8);
}
