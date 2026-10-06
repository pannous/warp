// P61 (user, 2026-10-05: "educate the user to use the be key word for definitions"): `is` compares; with a name defined
// nowhere `x is <value>` is an error that teaches `x be <value>`
use crate::common::fails_with;
use crate::is;

#[test]
fn is_with_an_undefined_name_teaches_be() {
	fails_with("x is 5; x", "x be 5");
	fails_with("x is 100 times [0]; x", "x be 100 times [0]");
	is!("x be 5; x is 5", 1);
	is!("x = 4; x is 5", 0);
	is!("s = 0; for x in [1, 2, 3] { if x is 2 { s = s + x } }; s", 2);
	is!("f(x) := x is 2; f(2)", 1);
}
