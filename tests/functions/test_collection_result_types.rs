//! Card list-result-type: a definition declaring a list or map result is a function like one declaring an int; the
//! result is checked, not converted (a text would become its characters, a map literal no map)
use crate::is;

#[test]
fn a_list_or_map_result_type_defines_the_function() {
	is!("f(x: int): list := [x]; f(2)#1", 2);
	is!("f(x: int): list := [x, x]; count(f(2))", 2);
	is!("f(x): map := {a: x}; f(3).a", 3);
	is!("f(x) as list := [x]; f(2)#1", 2);
}

#[test]
fn a_value_of_another_type_fails_the_declared_collection() {
	crate::common::fails_with("f(x: int): list := 3; f(2)", "list");
}
