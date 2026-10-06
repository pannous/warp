//! `not x` of an object, an empty block or a grouped empty value tests it as a Node

use crate::is;

#[test]
fn an_object_is_truthy() {
	is!("not {a:2}", false);
	is!("not {a:2 b:3}", false);
	is!("x={a:2}; not x", false);
	is!("!{a:2}", false);
}

#[test]
fn empty_values_are_falsy() {
	is!("not ()", true);
	is!("not {}", true);
	is!("not []", true);
	is!("not ({})", true);
}
