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

// Ruby blocks (card functions-ruby-yield): `yield v` calls the block the caller passes after the arguments
#[test]
fn test_ruby_yield_calls_the_block() {
	is!("total = 0\ndef each_twice\n  yield 1\n  yield 2\nend\neach_twice { |x| total += x }\ntotal", 3);
	is!("def twice(n)\n  yield n\n  yield n * 2\nend\nsum = 0\ntwice(3) { |x| sum += x }\nsum", 9);
	is!("def apply(n)\n  yield n\nend\napply(3) do |x| x * 2 end", 6);
	is!("def pair\n  yield 3, 4\nend\npair { |a, b| a * b }", 12);
	is!("def run\n  yield\nend\nrun { 5 }", 5);
}

// A block shares the variables it changes (P124): main's `total` is changed, not a copy
#[test]
fn test_block_changes_the_callers_variable() {
	is!("total = 0\ndef each_twice\n  yield 1\n  yield 2\nend\neach_twice { |x| total += x }\ntotal", 3);
	is!("total = 0\ndef g(h)\n  h(1)\n  h(2)\nend\ng(x => total += x)\ntotal", 3);
	is!("global total = 0\ndef g(h)\n  h(1)\n  h(2)\nend\ng(x => total += x)\ntotal", 3);
	is!("n = 0\ninc = () => { n += 1; n }\ninc()\ninc()", 2);
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

// Ruby's loops inside a `def … end` (card ruby-loop): `while c` … `end` and `loop do` … `end`
#[test]
fn test_ruby_loops_in_def() {
	is!("def f(n)\n  k = 0\n  while k < n\n    k += 1\n  end\n  k\nend\nf(3)", 3);
	is!("k = 0\nwhile k < 3\n  k += 1\nend\nk", 3);
	is!("def f\n  k = 0\n  loop do\n    k += 1\n    break if k > 2\n  end\n  k\nend\nf()", 3);
	is!("k = 0\nloop do\n  k += 1\n  break if k > 2\nend\nk", 3);
}
