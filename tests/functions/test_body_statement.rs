// A function body block of one braceless call, `def f(x){square x}` (Ruby/Kotlin style), is that call: it runs a
// library word, calls a user function, and an undefined word is an error, never the data `{square x}`
use crate::common::fails_with;
use crate::is;

#[test]
fn a_one_call_body_block_runs_the_call() {
	is!("def total(xs){sum xs}; total([1,2,3])", 6);
	is!("square(n):=n*n; def f(x){square x}; f(3)", 9);
	is!("def f(xs){ sum xs }; f([1,2])", 3);
}

#[test]
fn an_undefined_word_in_it_is_an_error() {
	fails_with("def f(x){square x}; f(3)", "undefined: square");
}
