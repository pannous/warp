// Functions are prefix operators (wiki/function.md): a function name among the juxtaposed arguments of another call
// takes the arguments after it, `square square 2` is `square(square(2))`, as in Haskell or Ruby `puts square 2`
use crate::is;

#[test]
fn a_function_name_among_the_arguments_takes_the_ones_after_it() {
	is!("square(n) := n*n; square square 2", 16);
	is!("square(n) := n*n; square(square 2)", 16);
	is!("inc(n) := n+1; square(n) := n*n; inc square 3", 10);
	is!("inc(n) := n+1; square(n) := n*n; square inc inc 1", 9);
	is!("add(a, b) := a+b; square(n) := n*n; add 1 square 3", 10);
}

#[test]
fn a_function_passed_as_a_value_stays_one() {
	is!("twice(f, x) := f(f(x)); square(n) := n*n; twice square 3", 81);
	is!("apply(f, x) := f(x); square(n) := n*n; apply(square, 4)", 16);
}
