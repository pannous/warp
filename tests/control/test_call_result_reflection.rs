//! Card name-function: `f()` (and `(f)`, the same call) is f's result, so `f().name` is the result's field; only the
//! bare name `f.name` and `function f` are the function itself (decision reflection)
use crate::is;
use warp::texts;

const PEOPLE: &str = "people() := [{name:\"a\", age:1}, {name:\"b\", age:2}]";

#[test]
fn a_field_of_a_call_reads_its_result() {
	is!(&format!("{PEOPLE}; people().name"), texts(vec!["a", "b"]));
	is!(&format!("{PEOPLE}; name of people()"), texts(vec!["a", "b"]));
	is!(&format!("{PEOPLE}; names of people()"), texts(vec!["a", "b"]));
	is!("person() := {name:\"Ann\", age:3}; person().name", "Ann");
}

#[test]
fn the_function_itself_keeps_its_name() {
	is!(&format!("{PEOPLE}; people.name"), "people");
	is!(&format!("{PEOPLE}; (function people).name"), "people");
}
