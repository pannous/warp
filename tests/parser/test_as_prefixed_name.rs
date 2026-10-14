//! A name starting with as_ (as_numbers, as_x) stays one name; the word as alone still casts
use crate::is;

#[test]
fn def_of_an_as_prefixed_name() {
	// was: (as def ((_x (v)) …)), undefined function as_x
	is!("def as_x(v) { v + 1 }; as_x(1)", 2);
}

#[test]
fn as_still_casts() {
	is!("x = 3.0; x as int", 3);
}
