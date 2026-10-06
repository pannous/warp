// Arithmetic on values whose kind is known only at run time (fields of a map parameter, parsed JSON): Floats when
// either is a Float, else exact Ints, decided by the values; was a "not an int" trap for floats
use crate::is;

#[test]
fn fields_of_a_map_parameter_compute_by_their_values() {
	is!("def f(m){ m.a * m.b }; f({a:sqrt(2), b:2})", 2.8284271247461903);
	is!("def f(m){ m.a + m.b }; f({a:1.5, b:2.25})", 3.75);
	is!("def f(m){ m.a + 1 }; f({a:2})", 3);
}

#[test]
fn a_def_body_reads_fields_of_its_parameters() {
	is!("def f(m){ m.a }; f({a:1})", 1);
	is!("def f(a, m){ m.a + a }; f(1, {a:2})", 3);
}

#[test]
fn a_text_in_runtime_kind_arithmetic_is_not_a_number() {
	crate::common::fails_with("def f(m){ m.a * 2 }; f({a:\"x\"})", "not a number");
	is!("def f(m){ m.a * 2 }; try f({a:\"x\"}) else 7", 7);
	is!("def f(m){ m.a + m.b }; f({a:2.5, b:sqrt(2)})", 3.914213562373095);
}

#[test]
fn as_float_of_a_runtime_value() {
	is!("def f(m){ m.a as float }; f({a:2.5})", 2.5);
	is!("def f(m){ m.a as float }; f({a:'7'})", 7.0);
	crate::common::fails_with("def f(m){ m.a as float }; f({a:\"x\"})", "invalid number");
}
