// Arithmetic on values whose kind is known only at run time (fields of a map parameter, parsed JSON): Floats when
// either is a Float, else exact Ints, decided by the values; was a "not an int" trap for floats
use warp::is;

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
