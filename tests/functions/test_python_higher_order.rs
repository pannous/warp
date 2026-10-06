// Python's colon definitions take functions as arguments like every other definition form
use crate::is;

#[test]
fn a_python_function_of_a_function() {
	is!("def apply(f, x): return f(x); apply(y => y * 2, 3)", 6);
	is!("def apply(f, *args): return f(*args); apply((a, b) => a * b, 3, 4)", 12);
	is!("def twice(f, x): return f(f(x)); twice(lambda y: y + 1, 5)", 7);
}

#[test]
fn python_parameter_markers_are_no_parameters() {
	is!("def f(a, *, b): return a + b; f(1, b=2)", 3); // keyword-only marker
	is!("def f(a, /, b): return a + b; f(1, 2)", 3); // positional-only marker
}
