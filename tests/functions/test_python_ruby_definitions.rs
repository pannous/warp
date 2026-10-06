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
