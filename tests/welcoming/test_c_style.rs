//! Welcoming C habits (samples/numbers/sin.warp, samples/numbers/sine.warp): `real f(real x) { … }` definitions and
//! `if (condition) statement` without braces
use crate::is;

#[test]
fn test_c_style_definition() {
	is!("real f(real x){ return x * 2 }; f(3)", 6);
	is!("int add(int a, int b) { return a + b }; add(2, 3)", 5);
	is!("float g(float x){ x = x - 1.5\nreturn x }\ng(4)", 2.5);
}

#[test] // `if(x<0) 7` was the list ` ø1 7`, `if (x<0) return 5` an error
fn test_if_parenthesized_condition_then_statement() {
	is!("x=-1; if(x<0) 7", 7);
	is!("x=1; if (x<0) 7 else 8", 8);
	is!("def f(x) { if (x<0) return 5\nreturn 6 }\nf(-1) * 10 + f(1)", 56);
}

#[test] // an `if` assigning a float variable was emitted as an Int statement: "assigns a float where an exact Int is expected"
fn test_if_assigning_a_float() {
	is!("g(x:float) := { if (x > 1) x = x - 1.5\nreturn x }\ng(4)", 2.5);
	is!("g(x:float) := { if x > 1 { x = x - 1.5 } else { x = 0.5 }\nreturn x }\ng(0) + g(4)", 3.0);
}

#[test] // card replacing-float: samples/numbers/sin.warp and sine.warp with `number` in place of `float`
fn test_number_parameter_holds_floats() {
	// a float write to a number parameter: "turn is a float where an exact Int is expected"
	is!("number f(number x){ turn = 2.5 as float\nx = x % turn\nreturn x }\nf(7)", 2.0);
	is!("f(x:number) := { x = x * 1.5f; x }; f(7)", 10.5);
	// a float argument: π / 2 became the exact ratio of its digits, and the sine polynomial a 200-digit fraction
	is!("f(x:number) := type(x)\nstr(f(π))", "class float");
	is!("f(x:number) := x; f(2) + f(1.5f)", 3.5);
	is!("f(x:number) := type(x)\nstr(f(1/3))", "class rational");
	let sine = std::fs::read_to_string("samples/numbers/sine.warp").unwrap().replace("float", "number");
	assert!(warp::wasm_emitter::eval(&sine).serialize().starts_with("1.0000000"));
	let sin = std::fs::read_to_string("samples/numbers/sin.warp").unwrap().replace("float", "number");
	is!(&sin, 841471);
}
