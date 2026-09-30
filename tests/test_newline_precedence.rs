use warp::*;

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
