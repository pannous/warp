// card int-spaces: an optional type reads the same with spaces around `=` as glued, `x: int? = 3` is `x:int?=3`
use crate::is;

#[test]
fn an_optional_type_before_a_spaced_assignment() {
	is!("x: int? = 3; x + 1", 4);
	is!("x : int? = ø; x = 2; x", 2);
	is!("name: text? = \"a\"; name", "a");
	is!("c = 1; c ? 2 : 3", 2); // a ternary `?` stays one
}
