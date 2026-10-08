//! card text-generally: `x as T?` keeps ø, any other value is cast to T; the result auto-unwraps (P179)

use crate::is;

#[test]
fn an_optional_cast_keeps_empty() {
	is!("x = ø; y = x as text?; y == ø", 1);
	is!("x = 3; (x as text?) + \"!\"", "3!");
	is!("x = \"4\"; (x as int?) + 1", 5);
	is!("f() = \"a\"; (f() as text?).upper()", "A"); // the call runs once, held by a hidden variable
}

#[test]
fn a_javascript_null_is_empty() {
	is!("use js JSON; x = JSON.parse(\"null\") as text?; x == ø", 1);
	is!("use js JSON; (JSON.parse(\"\\\"a\\\"\") as text?) + \"b\"", "ab");
}
