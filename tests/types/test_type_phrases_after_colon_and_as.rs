//! card cast-type: a collection type phrase after `:` or `as` is one type: `v as list of text`, a result
//! `h(xs): list of int := …`, `xs: map of int`; it read `(v as list) of text`, a lookup of text in the cast
use crate::is;

#[test]
fn a_cast_to_a_type_phrase() {
	is!("ys = [1, 2] as list of text\nys#1", "1");
	is!("ys = [\"1\", \"2\"] as list of int\nys#1 + ys#2", 3);
}

#[test]
fn a_declared_result_phrase() {
	is!("h(xs: list): list of int := xs\nh([4])#1", 4);
	is!("h(xs: list) as list of int := xs\nh([5])#1", 5);
}

#[test]
fn a_declared_variable_phrase() {
	is!("xs: list of int = [1, 2]\nxs#2", 2);
	is!("type Name = text\nf(xs: list of Name) := #xs\nf([\"a\", \"b\"])", 2);
	is!("m: map of int = {a: 1}\nm[\"a\"]", 1);
}
