//! Welcoming C habits (samples/sin.wasp, samples/sine.wasp): `real f(real x) { … }` definitions and
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
