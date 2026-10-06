use crate::is;

#[test]
fn test_fibonacci_auto_param_newline() {
	is!("fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\nfib(10)", 55);
}

#[test]
fn test_fibonacci_typed_newline() {
	is!("fib(n:int) = n < 2 ? n : fib(n - 1) + fib(n - 2)\nfib(10)", 55);
	is!("fib(n:number) = n < 2 ? n : fib(n - 1) + fib(n - 2)\nfib(10)", 55);
}

#[test]
fn test_fibonacci_auto_typed_newline() {
	is!("fib(n) = n < 2 ? n : fib(n - 1) + fib(n - 2)\nfib(10)", 55);
}

#[test]
#[should_panic]
fn newline_form_is_really_evaluated() {
	is!("fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\nfib(10)", 56);
}

#[test]
fn a_method_call_starting_the_next_line_continues_the_expression() {
	crate::is!("numbers = [1, 2, 3]\nr = numbers\n    .map(x => x*x)\n    .sum\nr", 14);
	crate::is!("x=[3 1 2]\nx\n  .sort", warp::ints(vec![1, 2, 3]));
	crate::is!("a=1\n.5", 0.5);
}

#[test]
fn then_and_else_branches_on_their_own_lines() {
	crate::is!("if 1 == 0 then\n    \"ab\"\nelse\n    \"cd\"", "cd");
	crate::is!("x = if 2 == 3 then\n    1\nelse if 2 == 2 then\n    2\nelse\n    3\nx", 2);
	crate::is!("elsewhere = 3\nelsewhere", 3);
}
