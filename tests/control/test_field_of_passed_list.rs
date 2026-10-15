//! Card field-list-passed: a list reaching an untyped parameter has no shape the compiler knows, so its field is
//! resolved at run time, the field of each element as for a list it knows (test_plural_field_of_elements)
use crate::is;
use warp::texts;

const PEOPLE: &str = "class P{name:text}; people=[P(\"a\"), P(\"b\")]";

#[test]
fn a_field_of_a_passed_list_of_instances() {
	is!(&format!("{PEOPLE}; g(xs) := xs.name; g(people)"), texts(vec!["a", "b"]));
	is!(&format!("{PEOPLE}; g(xs) := name of xs; g(people)"), texts(vec!["a", "b"]));
}

#[test]
fn a_plural_field_of_a_passed_list() {
	is!(&format!("{PEOPLE}; g(xs) := xs.names; g(people)"), texts(vec!["a", "b"]));
}

#[test]
fn a_passed_list_of_maps_of_two_entries() {
	is!("g(xs) := xs.name; g([{name:\"a\", age:1}, {name:\"b\", age:2}])", texts(vec!["a", "b"]));
}

#[test]
fn a_field_no_element_has_stays_missing() {
	crate::common::fails_with(&format!("{PEOPLE}; g(xs) := xs.age; g(people)"), "no field age");
}

/// card field-list-read: a list a function returns or JSON gives has no shape the compiler knows either
#[test]
fn a_field_of_a_returned_or_parsed_list() {
	is!("people() := [{name:\"a\", age:1}, {name:\"b\", age:2}]; p = people(); p.name", texts(vec!["a", "b"]));
	is!(
		"use json; p = json.parse(\"[{\\\"name\\\":\\\"a\\\"},{\\\"name\\\":\\\"b\\\"}]\"); p.name",
		texts(vec!["a", "b"])
	);
}
