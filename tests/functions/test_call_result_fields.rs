// A field of a call's value is read from the value the call gives: `f().name`, `lib.f(x).field` (card foreign-results)
use warp::error;
use crate::is;

#[test]
fn a_field_of_a_call_result_is_read_from_the_result() {
	is!("f() := {a:1 b:2}; f().b", 2);
	is!("f(x) := {v:x}; f(3).v + 1", 4);
	is!("person(name) := {name:name age:30}; person(\"Ann\").name", "Ann");
	is!("f() := {a:1}; f().c", error("no field c"));
}

#[test]
fn a_field_of_a_parenthesized_object_is_read() {
	is!("({a:1 b:2}).b", 2);
}

#[test]
fn counting_words_of_a_call_result_stay_counts() {
	is!("f() := [1,2]; f().count", 2);
	is!("g() := \"abc\"; g().length", 3);
}
