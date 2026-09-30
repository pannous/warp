//! Loops inside function bodies get their own temp locals: they used to reuse local 0 and 1 (parameter and first variable)

use warp::is;

#[test]
fn a_while_loop_in_a_function_keeps_its_variables() {
	is!("f(n):={ i=0; while (i < 3) {i++}; i }; f(4)", 3);
	is!("f(n):={ s=0; i=0; while i < n do {s+=i; i++}; s }; f(4)", 6);
}

#[test]
fn a_while_loop_in_a_function_keeps_its_parameter() {
	is!("f(n):={ s=0; while n > 0 do {s+=n; n--}; s }; f(4)", 10);
}

#[test]
fn a_multi_line_function_with_a_loop() {
	is!("sum(n) := {\n  s = 0\n  i = 0\n  while i < n {\n    s += i\n    i++\n  }\n  s\n}\nsum(4)", 6);
}
