// `abs(c)`: a variable in parentheses is that variable, not a call of a function c returning an Int; a float that a
// loop accumulates keeps its float through abs (card abs-loop)
use warp::wasm_emitter::eval;

#[test]
fn abs_of_float_accumulated_in_loop() {
	assert_eq!(eval("c = 0.0; for i in 1 to 3 { c = c + sin(i) }; abs(c) + 1"), eval("abs(sin(1) + sin(2) + sin(3)) + 1"));
	assert_eq!(eval("c = 0.0; for i in 1 to 3 { c += sin(i) }; abs(c)"), eval("abs(sin(1) + sin(2) + sin(3))"));
	assert_eq!(eval("c = 0.0; while c < 1 { c = c + sqrt(2.0) }; abs(c)"), eval("sqrt(2.0)"));
}

#[test]
fn parenthesized_float_variable_is_float() {
	assert_eq!(eval("c = 0.0; if 1 { c = c - sin(1.0) }; ‖(c)‖"), eval("sin(1.0)"));
	assert_eq!(eval("c = 0.0; if 1 { c = c - sin(1.0) }; d = c; abs(d)"), eval("sin(1.0)"));
}
