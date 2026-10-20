//! card or-once: a truthy left of `or` is evaluated once, also in a chain `a or b or c` of a function's bool result
//! (each `or` evaluated its left twice: lexer.warp is_name_char ran is_alphanumeric 64 times per letter)
use crate::is;

#[test]
fn a_truthy_left_of_or_runs_once() {
	is!("global n = 0; def f() { n += 1; 1 }; x = f() or 2; n", 1);
}

#[test]
fn a_chain_of_or_runs_its_first_test_once() {
	is!("global n = 0; def f(): bool { n += 1; true }; def g(): bool := f() or false or false or false; g(); n", 1);
	is!("global n = 0; def f(c: any): bool { n += 1; c == 1 }; def g(c: any): bool := f(c) or c == 2 or c == 3 or c == 4; g(1); n", 1);
	is!("def f(c: any): bool := c == 1 or c == 2 or c == 3; f(3)", true);
	is!("def f(c: any): bool := c == 1 or c == 2 or c == 3; f(5)", false);
}
