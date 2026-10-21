//! card pure-lane-holds: the name of a type value is the type's word, without the `class ` it prints with
use crate::is;

#[test]
fn the_name_of_a_type_is_its_word() {
	is!("type(3).name", "int");
	is!("t = type(\"ab\"); t.name", "text");
	is!("f(x: any) := type(x).name; f([])", "list");
	is!("f(x: any) := type(x).name; f(3)", "int");
}
