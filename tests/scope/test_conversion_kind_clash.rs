// Card kind-name (found by the type model, KNOWN_HOLES): a name first bound to a conversion `e as T` has T's kind, so
// P45 refuses another kind later as it does for `x = 1; x = "a"`
use crate::common::fails_with;
use crate::is;

#[test]
fn a_conversion_gives_its_type_s_kind() {
	fails_with("x = 3.7 as int; x = \"a\"", "x was an Int, is given a Text");
	fails_with("x = 3 as text; x = 5", "x was a Text, is given an Int");
	fails_with("x = 2 as bool; x = \"a\"", "x was a Bool, is given a Text");
}

#[test]
fn conversions_that_mix_stay_allowed() {
	is!("x = 3.7 as int; x = 2.5; x", 2.5);
	is!("x = 3 as float; x = 2; x", 2.0);
}
