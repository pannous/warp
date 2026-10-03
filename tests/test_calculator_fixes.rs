//! What samples/calculator.wasp needed: `else` on the next line, texts in comparisons, failing returns, run-time math
use warp::wasm_emitter::eval;
use warp::*;

#[test] // `}` newline `else if …` was not attached: the wrong branch ran
fn test_else_on_the_next_line() {
	is!("op = \"m\"; x = 0\nif op == \"p\" { x = 1 }\nelse if op == \"m\" { x = 2 }\nelse { x = 3 }\nx", 2);
	is!("op = \"z\"; x = 0\nif op == \"p\" { x = 1 }\nelse if op == \"m\" { x = 2 }\nelse { x = 3 }\nx", 3);
	is!("x = 0\nif 1 { x = 1 }\nx = x + 5\nx", 6);
}

#[test] // texts order by code points; a text-valued call equals a one-character literal by value
fn test_text_comparisons() {
	is!("s=\"ab\"; t=\"b\"; s < t", true);
	is!("s=\"b\"; s >= \"ab\"", true);
	is!("s = \"45\"; c = byte_slice(s, 0, 1); c >= \"0\" and c <= \"9\"", true);
	is!("def pk() := byte_slice(\"  x\", 0, 1)\npk() == \" \"", true);
}

#[test] // `return error(…)` does not make a number function a text one; in it the run fails with the message
fn test_error_returns() {
	is!("def f(n, x) { if n == \"s\" { return x * 2.5 }\nreturn error(\"unknown: \" + n) }\nf(\"s\", 2)", 5.0);
	assert!(format!("{:?}", eval("def f(n, x) { if n == \"s\" { return x * 2.5 }\nreturn error(\"unknown: \" + n) }\nf(\"q\", 2)")).contains("unknown: q"));
}

#[test] // a float function ending in `while true { … return … }`
fn test_float_function_ending_in_a_loop() {
	is!("def f(x) { y = x * 1.5\nwhile true { if y > 10 { return y }\ny = y * 2 } }\nf(2)", 12.0);
}

#[test] // sin(x) of a float computed at run time went nowhere (the data `sin`)
fn test_run_time_math() {
	is!("x = random()*0 + 1.0; round(sin(x) * 1000)", 841);
	is!("def f(x) := cos(x); round(f(0.0))", 1);
	is!("sin(π/2)", 1);
	is!("x = random()*0 + 2.0; round(ln(x) * 1000)", 693); // ln is libm's log
}
