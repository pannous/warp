//! Floats that live in declared fields, conditionals and globals (samples/raytracer.wasp)
use warp::*;

#[test] // `def dot(a, b) := a.x * b.x …` of `type V {x: float}` read the field as an Int: "not an int"
fn test_declared_float_field_in_arithmetic() {
	is!("type V { x: float, y: float }; def dot(a, b) := a.x * b.x + a.y * b.y; dot(V(1.5, 2), V(2, 1))", 5.0);
	is!("type S { r: float }; def f(s) { disc = s.r - 4; if disc < 0 { return 7 }; return disc }; f(S(8.5)) + f(S(1.0))", 11.5);
}

#[test] // max(0, x) of a float lowers to a ternary, which the float path did not emit
fn test_float_conditionals() {
	is!("x = random() * 0 + 0.75; max(0, x * 2.0)", 1.5);
	is!("x = random() * 0 + 0.5; (if x > 0.2 then x else 0.1) * 3.0", 1.5);
	is!("x = random() * 0 + 0.5; (x > 0.2 ? x : 0.1) * 3.0", 1.5);
}

#[test] // a global holding what a user function returns was typed Int before the function kinds were known
fn test_global_from_a_call() {
	is!("type V { x: float, y: float }; def vec(x, y) := V(x, y); global g = vec(1.5, 2); g.x", 1.5);
	is!("def f(x) := x * 1.5; global g = f(3); g", 4.5);
}

#[test] // a zero-argument user function in a float context: "undefined variable: f"
fn test_zero_argument_call_in_float_context() {
	is!("def f() { return random() }; f() >= 0", true);
	is!("def f() { return 1.5 }; f() * 2.0", 3.0);
}

#[test] // a function reading an outer variable set by a call got 0 (the capture was typed before the function kinds)
fn test_captured_variable_set_by_a_call() {
	is!("def g() := [3]; x = g(); def f() { return count(x) }; f()", 1);
	is!("def g() := 2.5; x = g(); def f() { return x * 2 }; f()", 5.0);
}
