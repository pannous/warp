//! A function returning a float on one path and an int on another is a float function: the int path gives its value
//! as a float (card function-returning, found by self-parser)
use crate::is;

#[test]
fn an_int_path_of_a_float_function_is_a_float() {
	is!(r#"def f(w) { if w.contains(".") { return float(w) }; int(w) }; f("12")"#, 12.0);
	is!(r#"def f(w) { if w.contains(".") { return float(w) }; int(w) }; f("1.5")"#, 1.5);
	is!(r#"def f(w) := w.contains(".") ? float(w) : int(w); f("12")"#, 12.0);
	is!(r#"def f(w) := w.contains(".") ? float(w) : int(w); f("2.5")"#, 2.5);
}
