// card error-field: a field of an Error is the Error, as its element is (`e#1`): `(try 10/0).count` fails with divide by
// zero instead of counting 1; the Error's own message stays a field
use crate::common::fails_with;
use crate::is;

#[test]
fn a_field_of_an_error_is_the_error() {
	fails_with("(try 10/0).count", "divide by zero");
	fails_with("e = try 10/0; count(e)", "divide by zero");
	fails_with("e = try 10/0; e.name", "divide by zero");
	is!("e = try 10/0; try e.count else 7", 7);
	is!("e = try 10/0; e.message", "divide by zero");
	fails_with("p = {a:1}; p.b", "no field b");
}
