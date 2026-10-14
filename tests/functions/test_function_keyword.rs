use crate::is;

#[test]
fn function_defines_like_def() {
	is!("function square(n){ n*n }; square 3", 9);
	is!("function square (n: number){ n*n }; square 3", 9);
	is!("function square (n: number){ n*n }; square(4)", 16);
	is!("def square(n){ n*n }; square 3", 9);
}

// card func-self-call-spaced: the body of `func name: …` calls itself without parentheses, as `fib := …` does
#[test]
fn func_colon_body_calls_itself_spaced() {
	is!("func fibonacci: if it < 2 then 1 else fibonacci(it-1) + fibonacci it - 2; fibonacci 5", 8);
	is!("func fibonacci: if it < 2 then 1 else fibonacci(it-1) + fibonacci(it-2); fibonacci 5", 8);
}
