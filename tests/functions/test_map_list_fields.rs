//! Card field-list: a field of a list of map literals is the field of each element, as of a list of instances
//! (test_list_fields), never the first element's alone
use crate::is;
use warp::texts;

const NAMES: &str = "m = [{name:\"a\", age:3}, {name:\"b\", age:5}]";

#[test]
fn a_field_of_a_list_of_maps_reads_each_element() {
	is!(&format!("{NAMES}; m.name"), texts(vec!["a", "b"]));
	is!(&format!("{NAMES}; name of m"), texts(vec!["a", "b"]));
}

#[test]
fn a_condition_names_the_fields_of_each_map() {
	is!(&format!("{NAMES}; count(m where age > 4)"), 1);
}

#[test]
fn one_element_keeps_its_field() {
	is!(&format!("{NAMES}; m#1.name"), "a");
}
