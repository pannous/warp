//! Python and Ruby definitions (cards functions-python, functions-ruby-def): `def f(*args): body`, `def f(**kw): body`
//! with a colon body, and Ruby's `def f(a, b: 2) … end`
use crate::is;

#[test]
fn test_python_rest_parameter_with_colon_body() {
	is!("def f(*args): return len(args)\nf(1, 2, 3)", 3);
	is!("def f(a, *rest): return a + len(rest)\nf(10, 2, 3)", 12);
}

#[test]
fn test_python_keyword_parameter() {
	is!("def f(**kw): return kw[\"a\"]\nf(a=1)", 1);
	is!("def f(**kw): return kw.b * 10\nf(a=1, b=2)", 20);
	is!("def f(x, **kw): return x + kw.a\nf(5, a=1)", 6);
	is!("def f(**kw): return count(kw)\nf()", 0);
}

#[test]
fn test_ruby_def_end() {
	is!("def f(a, b: 2) a * b end; f(3)", 6);
	is!("def f(a, b = 2)\n  a * b\nend\nf(3)", 6);
	is!("def g(x)\n  if x > 1 then x else 0 end\nend\ng(5)", 5);
	is!("def h\n  42\nend\nh()", 42);
}

#[test]
fn test_python_spread_object() {
	is!("def f(**kw): return kw.a\nm = {a: 7}\nf(**m)", 7);
	is!("def g(a, b, c): return a * 100 + b * 10 + c\nm = {b: 2, c: 3}\ng(1, **m)", 123);
	is!("def g(a, b, c): return a * 100 + b * 10 + c\nm = {b: 2, c: 3}\ng(1, c=9, **m)", 129);
}

#[test]
fn test_julia_definitions() {
	is!("function f(x; y=2) x*y end; f(3)", 6);
	is!("function f(x; y=2) x*y end; f(3; y=4)", 12);
	is!("function f(x)\n  x * 2\nend\nf(4)", 8);
	is!("f(x; y=2) = x*y; f(3, y=4)", 12);
}
