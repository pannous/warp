// P28 (user, 2026-10-05): `real x;` read before any assignment is the zero value of its type (Go)
use crate::is;

#[test]
fn a_declaration_reads_as_the_zero_value_of_its_type() {
	is!("real x; x*x", 0); // real is the exact type: an exact 0
	is!("int x; x=x+1; x", 1);
	is!("int n; n", 0);
	is!("float f; f + 1.5", 1.5);
}

#[test]
fn a_text_declaration_is_empty() {
	is!("text t; t + \"a\"", "a");
}
